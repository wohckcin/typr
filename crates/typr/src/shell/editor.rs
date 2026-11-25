//! Card editor opening, draft loading, save-then-leave coordination, and editor view wiring.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use nsh_core::prelude::*;
use typr_core::edits::deck_for_new_card;
use typr_core::store::{CardKind, Command, Id, MaskRect, Outcome, Snapshot};

use crate::screens::editor::{self, Cover, Diagram, Template, TemplateField};
use crate::screens::sidebar::Sidebar;

use super::Screen;
use super::store_bridge::StoreBridge;

/// Coordinates opening, creating, drafting, and saving cards in the editor over the store worker.
#[derive(Clone)]
pub(super) struct EditorController {
    bridge: StoreBridge,
    snapshot_signal: Signal<Snapshot>,
    screen: Signal<Screen>,
    attachments: PathBuf,
    draft: Rc<RefCell<Option<String>>>,
    save_error: Signal<Option<SharedString>>,
    save_in_flight: Rc<Cell<bool>>,
    leave_after_save: Rc<Cell<bool>>,
}

impl EditorController {
    pub(super) fn new(
        bridge: StoreBridge,
        snapshot_signal: Signal<Snapshot>,
        screen: Signal<Screen>,
        attachments: PathBuf,
    ) -> Self {
        let controller = Self {
            bridge,
            snapshot_signal,
            screen,
            attachments,
            draft: Rc::new(RefCell::new(None)),
            save_error: signal(None::<SharedString>),
            save_in_flight: Rc::new(Cell::new(false)),
            leave_after_save: Rc::new(Cell::new(false)),
        };
        // A card can vanish under the open editor (trashed elsewhere, or gone from a refreshed
        // snapshot). The route follows the snapshot rather than the render that noticed.
        effect(move || {
            if let Screen::Editor(id) = screen.get()
                && snapshot_signal.with(|snapshot| !snapshot.cards.iter().any(|card| card.id == id))
            {
                screen.set(Screen::Library);
            }
        });
        controller
    }

    /// Opens a card in the editor after loading any stored draft from the store worker.
    pub(super) fn open(&self, id: Id) {
        self.save_error.set(None);
        self.save_in_flight.set(false);
        self.leave_after_save.set(false);
        let draft_done = Rc::clone(&self.draft);
        let screen = self.screen;
        let sent = self.bridge.submit(
            Command::Draft { card: id },
            false,
            Some(Box::new(move |result| {
                let loaded = match result {
                    Ok(Outcome::Draft(text)) => text,
                    _ => None,
                };
                *draft_done.borrow_mut() = loaded;
                screen.set(Screen::Editor(id));
            })),
        );
        if sent.is_none() {
            *self.draft.borrow_mut() = None;
            self.screen.set(Screen::Editor(id));
        }
    }

    /// Creates a blank card in the active deck and opens it in the editor.
    pub(super) fn new_card(&self, sidebar: Sidebar) {
        let Some(deck) = deck_for_new_card(
            &self.snapshot_signal.get_untracked(),
            sidebar.selected_key().as_deref(),
        ) else {
            return;
        };
        let draft = Rc::clone(&self.draft);
        let save_error = self.save_error;
        let save_in_flight = Rc::clone(&self.save_in_flight);
        let leave_after_save = Rc::clone(&self.leave_after_save);
        let screen = self.screen;
        self.bridge.submit(
            Command::CreateCard {
                deck,
                markdown: String::new(),
            },
            true,
            Some(Box::new(move |result| {
                if let Ok(Outcome::Card(id)) = result {
                    *draft.borrow_mut() = None;
                    save_error.set(None);
                    save_in_flight.set(false);
                    leave_after_save.set(false);
                    screen.set(Screen::Editor(id));
                }
            })),
        );
    }

    /// Renders the editor for `id`, or runs `fallback` if the card disappeared from the snapshot.
    pub(super) fn render(&self, id: Id, fallback: impl FnOnce() -> AnyElement) -> AnyElement {
        let found = self
            .snapshot_signal
            .with(|snapshot| snapshot.cards.iter().find(|card| card.id == id).cloned());
        let Some(card) = found else {
            // The card went away under the editor. The effect above moves the screen back; until
            // that runs, show the library rather than nothing.
            return fallback();
        };
        let template = card.template.as_ref().map(|text| Template {
            text: text.clone(),
            fields: card
                .fields
                .iter()
                .map(|(name, value)| TemplateField {
                    name: name.clone(),
                    value: value.clone(),
                    computed: false,
                })
                .collect(),
            deck: None,
        });
        let diagram = (card.kind == CardKind::Diagram).then(|| Diagram {
            image: card.image.clone().unwrap_or_default(),
            caption: card.caption.clone().unwrap_or_default(),
            rows: card.masks.iter().map(Cover::of).collect(),
        });
        let draft = self.draft.borrow().clone();
        let send_save = self.save_dispatcher();
        let save_error = self.save_error;
        editor::editor()
            .saved(card.markdown.clone())
            .draft(draft.map(SharedString::from))
            .template(template)
            .diagram(diagram)
            .attachments(self.attachments.to_string_lossy().to_string())
            .save_error(move || save_error.get())
            .on_save({
                let send_save = send_save.clone();
                move |saved: &editor::CardText| {
                    send_save(Command::SaveCard {
                        id,
                        markdown: saved.text.clone(),
                    });
                }
            })
            .on_save_fields(move |saved: &editor::FieldValues| {
                send_save(Command::SetFields {
                    card: id,
                    fields: saved
                        .fields
                        .iter()
                        .map(|field| (field.name.clone(), field.value.clone()))
                        .collect(),
                });
            })
            .on_draft({
                let bridge = self.bridge.clone();
                let draft_ref = Rc::clone(&self.draft);
                move |draft: &editor::CardText| {
                    save_error.set(None);
                    *draft_ref.borrow_mut() = Some(draft.text.clone());
                    bridge.quiet_apply(Command::SetDraft {
                        card: id,
                        text: Some(draft.text.clone()),
                    });
                }
            })
            .on_leave({
                let save_in_flight = Rc::clone(&self.save_in_flight);
                let leave_after_save = Rc::clone(&self.leave_after_save);
                let screen = self.screen;
                move || {
                    if save_in_flight.get() {
                        leave_after_save.set(true);
                    } else if save_error.get_untracked().is_none() {
                        screen.set(Screen::Library);
                    }
                }
            })
            .on_discard({
                let bridge = self.bridge.clone();
                let draft_ref = Rc::clone(&self.draft);
                let screen = self.screen;
                move || {
                    // Discarding drops the stored draft, so the next open is the card.
                    *draft_ref.borrow_mut() = None;
                    bridge.quiet_apply(Command::SetDraft {
                        card: id,
                        text: None,
                    });
                    screen.set(Screen::Library);
                }
            })
            .on_add_mask({
                let bridge = self.bridge.clone();
                move |added: &editor::MaskAdded| {
                    let mask = added.mask;
                    bridge.apply(Command::AddMask {
                        card: id,
                        group: mask.group,
                        rect: MaskRect::new(mask.x, mask.y, mask.w, mask.h),
                    });
                }
            })
            .on_merge_masks({
                let bridge = self.bridge.clone();
                move |merged: &editor::GroupMerge| {
                    bridge.apply(Command::MergeMasks {
                        card: id,
                        group: merged.group,
                        rects: merged
                            .rects
                            .iter()
                            .map(|rect| MaskRect::new(rect.x, rect.y, rect.w, rect.h))
                            .collect(),
                    });
                }
            })
            .on_remove_mask_group({
                let bridge = self.bridge.clone();
                move |masked: &editor::MaskGroup| {
                    bridge.apply(Command::RemoveMaskGroup {
                        card: id,
                        group: masked.group,
                    });
                }
            })
            .on_clear_diagram({
                let bridge = self.bridge.clone();
                move || bridge.apply(Command::ClearDiagram { card: id })
            })
            .on_start_diagram({
                let bridge = self.bridge.clone();
                move |started: &editor::DiagramStart| {
                    bridge.apply(Command::SetDiagram {
                        card: id,
                        image: started.name.clone(),
                        caption: String::new(),
                    });
                }
            })
            .into_element()
    }

    fn save_dispatcher(&self) -> Rc<dyn Fn(Command)> {
        let bridge = self.bridge.clone();
        let save_in_flight = Rc::clone(&self.save_in_flight);
        let leave_after_save = Rc::clone(&self.leave_after_save);
        let draft_ref = Rc::clone(&self.draft);
        let save_error = self.save_error;
        let screen = self.screen;
        Rc::new(move |command: Command| {
            save_in_flight.set(true);
            save_error.set(None);
            let save_in_flight_done = Rc::clone(&save_in_flight);
            let leave_after_save_done = Rc::clone(&leave_after_save);
            let draft_done = Rc::clone(&draft_ref);
            bridge.submit(
                command,
                true,
                Some(Box::new(move |result| {
                    save_in_flight_done.set(false);
                    match result {
                        Ok(_) => {
                            *draft_done.borrow_mut() = None;
                            if leave_after_save_done.replace(false) {
                                screen.set(Screen::Library);
                            }
                        }
                        Err(error) => {
                            leave_after_save_done.set(false);
                            save_error.set(Some(
                                format!("Not saved: {}", crate::logging::describe(&error)).into(),
                            ));
                        }
                    }
                })),
            );
        })
    }
}
