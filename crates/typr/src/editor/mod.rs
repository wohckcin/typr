//! The card editor: the markdown buffer on the left, its rendering on the right.

use std::hash::{Hash, Hasher};

use rune::tok::{self, type_};
use rune::{El, ScrollSpec, TextEdit, col, memo, row, text, text_input};
use rune::{FileDialogOptions, Tx};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, Markdown, MarkdownStyle, root};

/// Kept for the lifetime of the app: a Kit rebuilds its palette and type ramp.
static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

use crate::attachments::{self, Attachments};
use typr_core::hidden::{self, Shape};
use typr_core::store::{CardKind, CardSummary, Id, MaskRect, MaskRow};

/// The app state an editor screen draws from.
pub trait Host: 'static {
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;

    /// Write the buffer to the library, answering whether it was stored.
    fn persist(&mut self) -> bool;

    /// Replace the tags on a card. A host with no store cannot, and says so by doing nothing.
    fn set_tags(&mut self, _card: Id, _tags: &[String]) {}

    /// The buffer changed. A host that keeps a draft writes it down here.
    fn note_typing(&mut self) {}

    /// Leave the editor, if the host has somewhere to go.
    fn close_editor(&mut self);

    /// Leave the editor, asking first when the buffer has unsaved work.
    fn leave_editor(&mut self) {
        self.close_editor();
    }

    /// Store a change to a diagram card's rectangles. A host with no store cannot.
    fn send_masks(&mut self, _card: Id, _change: &MaskChange) -> bool {
        false
    }
}

impl Host for Editor {
    fn editor(&self) -> &Editor {
        self
    }

    fn editor_mut(&mut self) -> &mut Editor {
        self
    }

    fn persist(&mut self) -> bool {
        self.save()
    }

    fn close_editor(&mut self) {}
}

/// One card being edited, and the text as it was last saved.
pub struct Editor {
    /// `None` while the card has never been saved.
    pub card: Option<Id>,
    pub buffer: TextEdit,
    /// The buffer as stored. A difference is what the unsaved indicator reports.
    pub saved: String,
    /// Whether the preview's parse is up to date with the buffer.
    memo: u64,
    /// Whether the leave confirmation is on screen.
    pub asking: bool,
    /// Whether the last save was refused because the card is empty.
    pub refused: bool,
    /// Why the last attachment did not happen, until another one succeeds.
    pub attach_error: Option<String>,
    /// The card's own reverse setting, or `None` when it inherits its deck's.
    pub reverse: Option<bool>,
    /// Where this card's pictures live.
    pub attachments: Attachments,
    /// Whether the card's review history is showing beside the preview.
    pub showing_history: bool,
    /// This card's review count and records, when the panel is open.
    pub history: crate::history::History,
    /// The tags, as one line the user types into.
    pub tags: TextEdit,
    /// The tag line for the input that shows it.
    /// The tag line as stored, so a change to it counts as unsaved.
    pub saved_tags: String,
    /// Whether this card asks in text or over a picture.
    pub kind: CardKind,
    /// The picture a diagram card asks about, as a name under the attachments folder.
    pub image: Option<String>,
    /// The picture as stored, so swapping it counts as unsaved.
    pub saved_image: Option<String>,
    /// What the picture shows.
    pub caption: TextEdit,
    /// The caption as stored.
    pub saved_caption: String,
    /// Every rectangle on the picture, one row each.
    pub masks: Vec<MaskRow>,
    /// The group the next rectangle is put in.
    pub next_group: u32,
    /// Which rectangle the keyboard fallback resizes.
    pub selected: usize,
    /// The size the picture pane was laid out at, in points.
    pub pane: (f32, f32),
}

/// A change to the rectangles on a diagram card, sent to the store.
#[derive(Clone, Debug, PartialEq)]
pub enum MaskChange {
    Add { group: u32, rect: MaskRect },
    Regroup { rects: Vec<MaskRect>, group: u32 },
    Remove { rect: MaskRect },
}

/// What the user asked for when leaving an unsaved card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Save,
    Discard,
    Stay,
}

/// Whether leaving needs the user's answer first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    Leave,
    Ask,
}

/// What to do once the user has answered, `None` when they stayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
}

impl crate::history::Host for Editor {
    fn history(&self) -> &crate::history::History {
        &self.history
    }

    fn history_mut(&mut self) -> &mut crate::history::History {
        &mut self.history
    }
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new(None, String::new())
    }
}

impl Editor {
    pub fn new(card: Option<Id>, text: String) -> Editor {
        let mut buffer = TextEdit::default();
        buffer.replace_text(text.clone());
        let memo = preview_key(card, &text);
        Editor {
            card,
            buffer,
            saved: text,
            memo,
            asking: false,
            refused: false,
            attach_error: None,
            reverse: None,
            attachments: Attachments::default(),
            showing_history: false,
            history: crate::history::History::default(),
            tags: TextEdit::default(),
            saved_tags: String::new(),
            kind: CardKind::Text,
            image: None,
            saved_image: None,
            caption: TextEdit::default(),
            saved_caption: String::new(),
            masks: Vec::new(),
            next_group: 1,
            selected: 0,
            pane: (0.0, 0.0),
        }
    }

    /// Take everything a card summary holds that this screen shows.
    pub fn load(&mut self, card: &CardSummary) {
        self.kind = card.kind;
        self.image = card.image.clone();
        self.saved_image = card.image.clone();
        let caption = card.caption.clone().unwrap_or_default();
        self.caption.replace_text(caption.clone());
        self.saved_caption = caption;
        self.masks = card.masks.clone();
        self.next_group = self.masks.iter().map(|mask| mask.group).max().unwrap_or(0) + 1;
        self.selected = 0;
    }

    /// Whether this card asks over a picture.
    pub fn diagram(&self) -> bool {
        self.kind == CardKind::Diagram
    }

    /// Switch between the two kinds. The text is kept either way, so a switch is undoable.
    pub fn set_kind(&mut self, kind: CardKind) {
        self.kind = kind;
    }

    /// Copy a chosen picture into the library and make this card a diagram of it.
    pub fn choose_image(&mut self, source: &std::path::Path) -> std::io::Result<()> {
        let name = attachments::attach(self.attachments.root(), source)?;
        self.image = Some(name);
        self.kind = CardKind::Diagram;
        self.attach_error = None;
        Ok(())
    }

    /// A rectangle from a drag, as fractions of the pane, or `None` when the drag was too small to
    /// be anything but a click.
    pub fn fraction(pane: (f32, f32), from: (f32, f32), to: (f32, f32)) -> Option<MaskRect> {
        let (w, h) = pane;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let left = from.0.min(to.0).clamp(0.0, w);
        let top = from.1.min(to.1).clamp(0.0, h);
        let right = from.0.max(to.0).clamp(0.0, w);
        let bottom = from.1.max(to.1).clamp(0.0, h);
        let rect = MaskRect::new(left / w, top / h, (right - left) / w, (bottom - top) / h);
        // A drag under one percent of the pane is a click that moved a little.
        if rect.w < 0.01 || rect.h < 0.01 {
            return None;
        }
        Some(rect)
    }

    /// Put a rectangle in a new group. Answers the change that has to be stored.
    pub fn add_rect(&mut self, rect: MaskRect) -> MaskChange {
        let group = self.next_group;
        self.next_group += 1;
        self.masks.push(MaskRow { group, rect });
        self.selected = self.masks.len() - 1;
        MaskChange::Add { group, rect }
    }

    /// Resize the selected rectangle by a fraction of the pane, keeping it inside the picture.
    pub fn resize_selected(&mut self, dw: f32, dh: f32) -> Option<MaskChange> {
        let mask = self.masks.get_mut(self.selected)?;
        let rect = MaskRect::new(
            mask.rect.x,
            mask.rect.y,
            (mask.rect.w + dw).clamp(0.01, 1.0 - mask.rect.x),
            (mask.rect.h + dh).clamp(0.01, 1.0 - mask.rect.y),
        );
        mask.rect = rect;
        let group = mask.group;
        Some(MaskChange::Add { group, rect })
    }

    /// Move the selection to the next rectangle, so the keyboard can reach every one.
    pub fn select_next(&mut self) {
        if self.masks.is_empty() {
            self.selected = 0;
        } else {
            self.selected = (self.selected + 1) % self.masks.len();
        }
    }

    /// Merge the selected rectangle into the group of the one before it.
    pub fn merge_selected(&mut self) -> Option<MaskChange> {
        if self.selected == 0 || self.selected >= self.masks.len() {
            return None;
        }
        let group = self.masks[self.selected - 1].group;
        self.masks[self.selected].group = group;
        let rects: Vec<MaskRect> = self
            .masks
            .iter()
            .filter(|mask| mask.group == group)
            .map(|mask| mask.rect)
            .collect();
        Some(MaskChange::Regroup { rects, group })
    }

    /// Take the selected rectangle off the picture.
    pub fn drop_selected(&mut self) -> Option<MaskChange> {
        if self.masks.is_empty() {
            return None;
        }
        let mask = self.masks.remove(self.selected.min(self.masks.len() - 1));
        self.selected = self.selected.min(self.masks.len().saturating_sub(1));
        Some(MaskChange::Remove { rect: mask.rect })
    }

    /// The card's reverse state as one word, so the header reads the same in every case.
    pub fn reverse_label(&self) -> &'static str {
        match self.reverse {
            Some(true) => "Reverse: on",
            Some(false) => "Reverse: off",
            None => "Reverse: Inherit",
        }
    }

    /// Record the card's own reverse setting, which the store keeps on the card.
    pub fn set_reverse(&mut self, reverse: Option<bool>) {
        self.reverse = reverse;
    }

    /// Hold the tags the card carries now, so the field starts where the card is.
    pub fn load_tags(&mut self, tags: &[String]) {
        let line = tags.join(", ");
        self.tags.replace_text(line.clone());
        self.saved_tags = line;
    }

    /// The tag line as typed, for the input that shows it.
    pub fn tags_text(&self) -> &TextEdit {
        &self.tags
    }

    /// The tags the field holds, split on commas or spaces and de-duplicated, keeping the first
    /// spelling of each so `Grammar` and `grammar` cannot both be added.
    pub fn tag_list(&self) -> Vec<String> {
        let mut kept: Vec<String> = Vec::new();
        for raw in self
            .tags
            .text
            .split(|character: char| character == ',' || character.is_whitespace())
        {
            let tag = raw.trim();
            if tag.is_empty() {
                continue;
            }
            let folded = tag.to_lowercase();
            if kept.iter().any(|own| own.to_lowercase() == folded) {
                continue;
            }
            kept.push(tag.to_string());
        }
        kept
    }

    /// Whether the buffer or the tag line holds something the card does not.
    pub fn dirty(&self) -> bool {
        self.buffer.text != self.saved
            || self.tags.text != self.saved_tags
            || self.image != self.saved_image
            || (self.diagram() && self.caption.text.trim() != self.saved_caption)
    }

    /// The sides the buffer splits into right now, so the preview and the save agree.
    pub fn sides(&self) -> Vec<String> {
        typr_core::sides::split(&self.buffer.text)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// How many prompts the card will have once it is stored: numbered hidden-text groups
    /// count one each, two or more sides count one, and a note counts none.
    pub fn prompts(&self) -> usize {
        hidden::shape(&self.buffer.text).prompt_count()
    }

    /// Keep the preview's parse only as long as its source, so editing another card cannot
    /// invalidate this one's.
    pub fn settle(&mut self) {
        self.memo = preview_key(self.card, &self.buffer.text);
    }

    /// Whether there is anything to store: a card of nothing but whitespace is refused.
    pub fn saveable(&self) -> bool {
        // A diagram card is asked over its picture, so it needs no text to be worth keeping.
        if self.diagram() {
            return self.image.is_some();
        }
        self.sides().iter().any(|side| !side.trim().is_empty())
    }

    /// Why the last save or attachment did not happen, so the editor can say so.
    pub fn refusal(&self) -> Option<&str> {
        if self.refused && !self.saveable() {
            Some("This card is empty")
        } else {
            self.attach_error.as_deref()
        }
    }

    /// Copy a chosen file beside the library and write its reference where the cursor is. A
    /// failure is kept on the editor, because the file dialog answers on its own thread.
    pub fn attach_image(&mut self, source: &std::path::Path) -> std::io::Result<String> {
        match self.copy_in(source) {
            Ok(name) => {
                self.attach_error = None;
                Ok(name)
            }
            Err(error) => {
                self.attach_error = Some(format!("The image was not attached: {error}"));
                Err(error)
            }
        }
    }

    fn copy_in(&mut self, source: &std::path::Path) -> std::io::Result<String> {
        let name = attachments::attach(self.attachments.root(), source)?;
        let alt = name
            .rsplit_once('.')
            .map_or(name.as_str(), |(stem, _)| stem);
        self.buffer.insert(&format!("![{alt}]({name})"));
        self.settle();
        Ok(name)
    }

    /// The draft belongs to the card, or to the unsaved new card.
    pub fn draft_key(&self) -> Id {
        self.card.unwrap_or(0)
    }

    /// Mark the buffer as stored. The caller persists it; nothing is stored for an empty card.
    pub fn save(&mut self) -> bool {
        if !self.saveable() {
            self.refused = true;
            return false;
        }
        self.refused = false;
        self.saved = self.buffer.text.clone();
        self.saved_tags = self.tags.text.trim().to_string();
        self.saved_image = self.image.clone();
        self.saved_caption = self.caption.text.trim().to_string();
        self.settle();
        true
    }

    /// Put a draft back into a card that has nothing typed into it yet.
    pub fn restore_draft(&mut self, text: &str) {
        if text.is_empty() || !self.buffer.text.trim().is_empty() {
            return;
        }
        self.buffer.replace_text(text.to_string());
        self.settle();
    }

    /// Ask before closing an unsaved card; a clean one closes straight away.
    pub fn request_leave(&mut self) -> Leaving {
        if self.dirty() {
            self.asking = true;
            Leaving::Ask
        } else {
            Leaving::Leave
        }
    }

    /// Answer the confirmation. Discarding puts the buffer back as it was saved.
    #[must_use]
    pub fn resolve(&mut self, choice: Choice) -> Option<Answer> {
        self.asking = false;
        match choice {
            Choice::Save => Some(Answer::Save),
            Choice::Discard => {
                self.buffer.replace_text(self.saved.clone());
                self.settle();
                Some(Answer::Discard)
            }
            Choice::Stay => None,
        }
    }

    /// The identity of the current parse: the card and the exact text.
    pub fn memo(&self) -> u64 {
        self.memo
    }
}

/// What the preview draws: a card that hides text is drawn with its first group masked, so the
/// author sees the question a session will ask, not the answer.
pub fn preview_body(text: &str) -> String {
    let shape = hidden::shape(text);
    hidden::masked_all(text, &shape)
}

/// The line under the preview: prompts for a card that hides text, sides for a card that does
/// not, because "two sides" would be wrong for two numbered groups on one side.
pub fn caption(text: &str) -> String {
    match hidden::shape(text) {
        Shape::Hidden { .. } => match hidden::shape(text).prompt_count() {
            1 => "1 prompt".to_string(),
            n => format!("{n} prompts"),
        },
        Shape::Sides(sides) => match sides.iter().filter(|side| !side.trim().is_empty()).count() {
            0 => "Empty card".to_string(),
            1 => "One side: this is a note and never queues".to_string(),
            n => format!("{n} sides"),
        },
    }
}

/// A parse is chosen by the card and the text, nothing else, so one card's edit leaves every
/// other card's parse alone.
pub fn preview_key(card: Option<Id>, text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    card.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

fn markdown(attachments: &Attachments, text: &str, key: u64) -> El {
    let images = attachments.clone();
    memo("preview", key, move || {
        Markdown::new(text.to_string())
            .images(move |src| images.picture(src))
            .render(&MarkdownStyle::neutral(true))
    })
}

fn source_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    text_input(&s.editor().buffer, type_::BODY.lh(22.0))
        .key("sides")
        .grow(1.0)
        .placeholder(
            "Write the card. A line of --- starts the next side.",
            t.text_secondary,
        )
        .on_edit(|s: &mut S| &mut s.editor_mut().buffer)
        .on_input_change(|s: &mut S, _| {
            s.editor_mut().settle();
            s.note_typing();
        })
}

fn preview_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let editor = s.editor();
    let drawn = preview_body(&editor.buffer.text);
    col()
        .key("preview-pane")
        .grow(1.0)
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .gap(8.0)
        .padding(16.0)
        .fill(t.card)
        .child(if editor.saveable() {
            markdown(&editor.attachments, &drawn, editor.memo())
        } else {
            text("Nothing to save yet. Write the front of the card.")
                .key("preview-empty")
                .color(t.text_secondary)
        })
        .child(
            text(caption(&editor.buffer.text))
                .key("preview-caption")
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
}

fn button<S: Host>(label: &'static str, variant: ButtonVariant, choice: Choice) -> El {
    Button::new(label)
        .variant(variant)
        .on_press(move |s: &mut S| {
            if let Some(answer) = s.editor_mut().resolve(choice) {
                match answer {
                    Answer::Save => {
                        s.persist();
                        s.close_editor();
                    }
                    Answer::Discard => s.close_editor(),
                }
            }
        })
        .build(&KIT)
        .key(match choice {
            Choice::Save => "leave-save",
            Choice::Discard => "leave-discard",
            Choice::Stay => "leave-stay",
        })
}

fn save_button<S: Host>() -> El {
    Button::new("Save")
        .variant(ButtonVariant::Submit)
        .on_press(|s: &mut S| {
            s.persist();
        })
        .build(&KIT)
        .key("save-card")
}

/// The way out of the editor: the library, asking first if the buffer is dirty.
fn back_button<S: Host>() -> El {
    Button::new("Library")
        .variant(ButtonVariant::Ghost)
        .on_press(|s: &mut S| {
            s.leave_editor();
        })
        .build(&KIT)
        .key("back-to-library")
}

/// The tag line, which stores what it holds as soon as the user presses Enter.
fn mode_button<S: Host>(label: &'static str, kind: CardKind, key: &'static str) -> El {
    Button::new(label)
        .variant(ButtonVariant::Ghost)
        .on_press(move |s: &mut S| s.editor_mut().set_kind(kind))
        .build(&KIT)
        .key(key)
}

fn tag_field<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    text_input(s.editor().tags_text(), type_::LABEL)
        .key("tags")
        .w(240.0)
        .single_line()
        .placeholder("tags, comma separated", t.text_secondary)
        .on_edit(|s: &mut S| &mut s.editor_mut().tags)
        .on_input_change(|s: &mut S, _| s.note_typing())
        .on_submit(|s: &mut S, _| {
            let Some(card) = s.editor().card else {
                return;
            };
            let tags = s.editor().tag_list();
            s.set_tags(card, &tags);
            let line = s.editor().tags.text.clone();
            s.editor_mut().saved_tags = line;
        })
}

fn attach_button<S: Host>() -> El {
    Button::new("Attach image")
        .variant(ButtonVariant::Ghost)
        .on_press_tx(|_: &mut S, tx: &mut Tx| {
            let options = FileDialogOptions {
                title: "Attach image".into(),
                ..FileDialogOptions::default()
            }
            .filter(
                "Images",
                &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"],
            );
            tx.open_file(options, |s: &mut S, result, _| {
                if let Ok(Some(path)) = result {
                    // The error is kept on the editor and drawn in the header.
                    let _ = s.editor_mut().attach_image(&path);
                }
            });
        })
        .build(&KIT)
        .key("attach-image")
}

fn confirmation<S: Host>(s: &S, layer: &mut Vec<El>) {
    let buttons = row()
        .key("leave-buttons")
        .w_full()
        .padding(16.0)
        .gap(8.0)
        .justify_end()
        .child(button::<S>("Stay", ButtonVariant::Ghost, Choice::Stay))
        .child(button::<S>(
            "Discard",
            ButtonVariant::Ghost,
            Choice::Discard,
        ))
        .child(button::<S>("Save", ButtonVariant::Submit, Choice::Save));
    Dialog::new("leave", "Unsaved changes")
        .description("Save this card before closing it?")
        .fit()
        .open(s.editor().asking)
        .on_close(|s: &mut S| {
            // Clicking outside the dialog is staying: the answer is deliberately unused.
            let _ = s.editor_mut().resolve(Choice::Stay);
        })
        .child(buttons)
        .build(&KIT, layer);
}

/// The editor screen. The preview is rebuilt from the buffer on the same frame, so there is no
/// mode to switch into.
/// The picture a diagram card asks about, the rectangles drawn on it, and the controls that make
/// and shape them without a pointer.
fn diagram_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let editor = s.editor();
    let (pane_w, pane_h) = editor.pane;
    let mut pane = col()
        .key("diagram-pane")
        .w_full()
        .max_w(400.0)
        .gap(8.0)
        .on_layout(|s: &mut S, rect| s.editor_mut().pane = (rect[2], rect[3]))
        .on_drag(|s: &mut S, drag: &mut rune::Drag, _: &mut rune::Tx| {
            // A rectangle is the drag's own corners read as fractions of the pane the gesture was
            // drawn on, which the drag reports in its own element's points.
            if drag.phase != rune::Phase::Up || drag.cancelled() {
                return;
            }
            let pane = (drag.w, drag.h);
            let Some(rect) = Editor::fraction(pane, (drag.start_x, drag.start_y), (drag.x, drag.y))
            else {
                return;
            };
            let change = s.editor_mut().add_rect(rect);
            if let Some(card) = s.editor().card {
                s.send_masks(card, &change);
            }
        });

    match editor
        .image
        .as_ref()
        .and_then(|name| editor.attachments.picture(name))
    {
        Some(picture) => {
            let mut drawn = rune::el().role("image").w_full().padding(0.0);
            if let Some([w, _]) = picture.size() {
                drawn = drawn.max_w(w as f32);
            }
            pane = pane.child(drawn.pixels(picture.pixels()));
        }
        None => {
            pane = pane.child(
                text("Choose a picture for this card.")
                    .key("no-picture")
                    .font(type_::LABEL)
                    .color(t.text_secondary),
            );
        }
    }

    // One outline per rectangle, placed against the pane as it was last laid out, labelled with the
    // group it answers in.
    for (index, mask) in editor.masks.iter().enumerate() {
        pane = pane.child(
            col()
                .key(format!("shape-{index}"))
                .absolute(mask.rect.x * pane_w, mask.rect.y * pane_h)
                .size(mask.rect.w * pane_w, mask.rect.h * pane_h)
                .radius(2.0)
                .fill(tok::accent(1, true).alpha(0.12))
                .border(1.0, tok::accent(1, true))
                .child(
                    text(format!("{}", mask.group))
                        .font(type_::LABEL)
                        .color(tok::accent(1, true)),
                ),
        );
    }

    let controls = row()
        .key("diagram-controls")
        .gap(6.0)
        .child(
            Button::new("Choose picture")
                .variant(ButtonVariant::Ghost)
                .on_press_tx(|_: &mut S, tx: &mut Tx| {
                    tx.open_file(
                        FileDialogOptions {
                            title: "Choose a picture".to_string(),
                            ..Default::default()
                        }
                        .filter(
                            "Images",
                            &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"],
                        ),
                        |s: &mut S, result, _| {
                            if let Ok(Some(path)) = result
                                && let Err(error) = s.editor_mut().choose_image(&path)
                            {
                                s.editor_mut().attach_error = Some(error.to_string());
                            }
                        },
                    );
                })
                .build(&KIT)
                .key("choose-image"),
        )
        .child(
            Button::new("Add rectangle")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| {
                    // The keyboard fallback places a rectangle in the middle of the picture.
                    let change = s.editor_mut().add_rect(MaskRect::new(0.35, 0.35, 0.3, 0.3));
                    if let Some(card) = s.editor().card {
                        s.send_masks(card, &change);
                    }
                })
                .build(&KIT)
                .key("add-rect"),
        )
        .child(
            Button::new("Next rectangle")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.editor_mut().select_next())
                .build(&KIT)
                .key("next-rect"),
        )
        .child(
            Button::new("Wider")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| {
                    if let Some(change) = s.editor_mut().resize_selected(0.05, 0.0)
                        && let Some(card) = s.editor().card
                    {
                        s.send_masks(card, &change);
                    }
                })
                .build(&KIT)
                .key("wider-rect"),
        )
        .child(
            Button::new("Taller")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| {
                    if let Some(change) = s.editor_mut().resize_selected(0.0, 0.05)
                        && let Some(card) = s.editor().card
                    {
                        s.send_masks(card, &change);
                    }
                })
                .build(&KIT)
                .key("taller-rect"),
        )
        .child(
            Button::new("Merge")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| {
                    if let Some(change) = s.editor_mut().merge_selected()
                        && let Some(card) = s.editor().card
                    {
                        s.send_masks(card, &change);
                    }
                })
                .build(&KIT)
                .key("merge-rect"),
        )
        .child(
            Button::new("Remove")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| {
                    if let Some(change) = s.editor_mut().drop_selected()
                        && let Some(card) = s.editor().card
                    {
                        s.send_masks(card, &change);
                    }
                })
                .build(&KIT)
                .key("remove-rect"),
        );

    pane.child(controls).child(
        text_input(&editor.caption, type_::LABEL)
            .key("caption-line")
            .w_full()
            .single_line()
            .placeholder("What the picture shows", t.text_secondary)
            .on_edit(|s: &mut S| &mut s.editor_mut().caption)
            .on_input_change(|s: &mut S, _| s.note_typing()),
    )
}

pub fn view<S: Host + crate::history::Host>(s: &S) -> El {
    let t = tok::theme(true);
    let unsaved = s.editor().dirty();
    let refusal = s.editor().refusal();
    let header = row()
        .key("editor-header")
        .h(44.0)
        .px(16.0)
        .center_y()
        .gap(8.0)
        .child(
            text(if unsaved { "Unsaved changes" } else { "Saved" })
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(
            text(refusal.unwrap_or_default())
                .key("refusal")
                .font(type_::LABEL)
                .color(if refusal.is_some() {
                    KIT.c.danger.fg
                } else {
                    t.text_secondary
                }),
        )
        .child(tag_field::<S>(s))
        .child(
            text(s.editor().reverse_label())
                .key("reverse-state")
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(mode_button::<S>("Text", CardKind::Text, "mode-text"))
        .child(mode_button::<S>(
            "Diagram",
            CardKind::Diagram,
            "mode-diagram",
        ))
        .child(rune::el().grow(1.0))
        .child(back_button::<S>())
        .child(attach_button::<S>())
        .child(save_button::<S>());
    let mut layer = Vec::new();
    confirmation::<S>(s, &mut layer);
    let body = row().w_full().h_full().fill(t.bg).child(
        col().w_full().h_full().child(header).child(
            row()
                .grow(1.0)
                .h_full()
                .padding(12.0)
                .gap(12.0)
                .child(
                    col()
                        .key("source")
                        .grow(1.0)
                        .h_full()
                        .child(if s.editor().diagram() {
                            diagram_pane::<S>(s)
                        } else {
                            source_pane::<S>(s)
                        }),
                )
                .child(preview_pane::<S>(s))
                .children(s.editor().showing_history.then(|| {
                    col()
                        .key("history-panel")
                        .w(320.0)
                        .h_full()
                        .padding(12.0)
                        .child(crate::history::view::<S>(s))
                })),
        ),
    );
    root(body, layer)
}

#[cfg(test)]
mod tests;
