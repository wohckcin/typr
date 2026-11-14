//! The card editor: the markdown buffer on the left, its rendering on the right.

use std::hash::{Hash, Hasher};

use rune::tok::{self, type_};
use rune::{El, ScrollSpec, TextEdit, col, memo, row, text, text_input};
use rune::{FileDialogOptions, Tx};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, Markdown, MarkdownStyle, root};

/// Kept for the lifetime of the app: a Kit rebuilds its palette and type ramp.
static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

use crate::attachments::{self, Attachments};
use typr_core::store::Id;

/// The app state an editor screen draws from.
pub trait Host: 'static {
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;

    /// Write the buffer to the library, answering whether it was stored.
    fn persist(&mut self) -> bool;

    /// Replace the tags on a card. A host with no store cannot, and says so by doing nothing.
    fn set_tags(&mut self, _card: Id, _tags: &[String]) {}

    /// Leave the editor, if the host has somewhere to go.
    fn close_editor(&mut self);
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
    /// Where this card's pictures live.
    pub attachments: Attachments,
    /// Whether the card's review history is showing beside the preview.
    pub showing_history: bool,
    /// This card's review count and records, when the panel is open.
    pub history: crate::history::History,
    /// The tags, as one line the user types into.
    pub tags: TextEdit,
    /// The tag line as stored, so a change to it counts as unsaved.
    pub saved_tags: String,
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
            attachments: Attachments::default(),
            showing_history: false,
            history: crate::history::History::default(),
            tags: TextEdit::default(),
            saved_tags: String::new(),
        }
    }

    /// Hold the tags the card carries now, so the field starts where the card is.
    pub fn load_tags(&mut self, tags: &[String]) {
        let line = tags.join(", ");
        self.tags.replace_text(line.clone());
        self.saved_tags = line;
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
        self.buffer.text != self.saved || self.tags.text != self.saved_tags
    }

    /// The sides the buffer splits into right now, so the preview and the save agree.
    pub fn sides(&self) -> Vec<String> {
        typr_core::sides::split(&self.buffer.text)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// Keep the preview's parse only as long as its source, so editing another card cannot
    /// invalidate this one's.
    pub fn settle(&mut self) {
        self.memo = preview_key(self.card, &self.buffer.text);
    }

    /// Whether there is anything to store: a card of nothing but whitespace is refused.
    pub fn saveable(&self) -> bool {
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
        self.settle();
        true
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
        .on_input_change(|s: &mut S, _| s.editor_mut().settle())
}

fn preview_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let editor = s.editor();
    let sides = typr_core::sides::split(&editor.buffer.text).len();
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
            markdown(&editor.attachments, &editor.buffer.text, editor.memo())
        } else {
            text("Nothing to save yet. Write the front of the card.")
                .key("preview-empty")
                .color(t.text_secondary)
        })
        .child(
            text(match sides {
                0 => "Empty card".to_string(),
                1 => "One side: this is a note and never queues".to_string(),
                n => format!("{n} sides"),
            })
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

/// The tag line, which stores what it holds as soon as the user presses Enter.
fn tag_field<S: Host>() -> El {
    let t = tok::theme(true);
    text_input(&TextEdit::default(), type_::LABEL)
        .key("tags")
        .w(240.0)
        .single_line()
        .placeholder("tags, comma separated", t.text_secondary)
        .on_edit(|s: &mut S| &mut s.editor_mut().tags)
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
        .child(tag_field::<S>())
        .child(rune::el().grow(1.0))
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
                        .child(source_pane::<S>(s)),
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
