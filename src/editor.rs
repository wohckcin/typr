//! The card editor: the markdown buffer on the left, its rendering on the right.

use std::hash::{Hash, Hasher};

use rune::tok::{self, type_};
use rune::{El, ScrollSpec, TextEdit, col, memo, row, text, text_input};
use rune_kit::{Markdown, MarkdownStyle};

use crate::store::Id;

/// The app state an editor screen draws from.
pub trait Host: 'static {
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
}

impl Host for Editor {
    fn editor(&self) -> &Editor {
        self
    }

    fn editor_mut(&mut self) -> &mut Editor {
        self
    }
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
        Editor { card, buffer, saved: text, memo }
    }

    /// Whether the buffer holds something the card does not.
    pub fn dirty(&self) -> bool {
        self.buffer.text != self.saved
    }

    /// The sides the buffer splits into right now, so the preview and the save agree.
    pub fn sides(&self) -> Vec<String> {
        crate::sides::split(&self.buffer.text).into_iter().map(str::to_string).collect()
    }

    /// Keep the preview's parse only as long as its source, so editing another card cannot
    /// invalidate this one's.
    pub fn settle(&mut self) {
        self.memo = preview_key(self.card, &self.buffer.text);
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

fn markdown(text: &str, key: u64) -> El {
    memo("preview", key, || Markdown::new(text.to_string()).render(&MarkdownStyle::neutral(true)))
}

fn source_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    text_input(&s.editor().buffer, type_::BODY.lh(22.0))
        .key("sides")
        .grow(1.0)
        .placeholder("Write the card. A line of --- starts the next side.", t.text_secondary)
        .on_edit(|s: &mut S| &mut s.editor_mut().buffer)
        .on_input_change(|s: &mut S, _| s.editor_mut().settle())
}

fn preview_pane<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let editor = s.editor();
    let sides = crate::sides::split(&editor.buffer.text).len();
    col()
        .key("preview-pane")
        .grow(1.0)
        .h_full()
        .scroll(ScrollSpec { stick_bottom: false })
        .gap(8.0)
        .padding(16.0)
        .fill(t.card)
        .child(markdown(&editor.buffer.text, editor.memo()))
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

/// The editor screen. The preview is rebuilt from the buffer on the same frame, so there is no
/// mode to switch into.
pub fn view<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let unsaved = s.editor().dirty();
    let header = row()
        .key("editor-header")
        .h(44.0)
        .px(16.0)
        .center_y()
        .gap(8.0)
        .child(text(if unsaved { "Unsaved changes" } else { "Saved" }).font(type_::LABEL).color(t.text_secondary));
    row()
        .w_full()
        .h_full()
        .fill(t.bg)
        .child(col().w_full().h_full().child(header).child(
            row()
                .grow(1.0)
                .h_full()
                .padding(12.0)
                .gap(12.0)
                .child(col().key("source").grow(1.0).h_full().child(source_pane::<S>(s)))
                .child(preview_pane::<S>(s)),
        ))
}

#[cfg(test)]
mod tests;
