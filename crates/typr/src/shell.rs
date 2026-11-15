//! The NSH app's root and its shell: a sidebar that groups what the library holds, beside the
//! work area. Pages and screens join the shell as they are ported.

use nsh_core::prelude::*;

use crate::ui::{surface, tokens};

/// What the sidebar groups. Its signals stay private: writes go through the methods.
#[derive(Clone, Copy)]
pub struct Library {
    decks: Signal<Vec<&'static str>>,
    tags: Signal<Vec<&'static str>>,
    views: Signal<Vec<&'static str>>,
}

impl Library {
    /// A library holding the sample groups a fresh install shows.
    pub fn sample() -> Self {
        Self {
            decks: signal(vec!["Spanish", "Biology"]),
            tags: signal(vec!["verbs"]),
            views: signal(Vec::new()),
        }
    }
}

fn library() -> Library {
    use_context::<Library>().expect("the root provides the library")
}

/// One heading of the sidebar with its count, as the Design Guides ask: the group name, then how many.
fn group(name: &'static str, count: impl Fn() -> usize + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .justify_between()
        .child(text(name).text_sm().text_color(move || tokens::muted_foreground()))
        .child(text(move || count().to_string()).text_sm().text_color(move || tokens::muted_foreground()))
}

/// The sidebar: Decks, Tags and Views, each with a count, and an empty group still named.
fn sidebar() -> impl IntoElement {
    let library = library();
    view()
        .w(px(220.0))
        .p_3()
        .gap_3()
        .border_r_1()
        .border_color(move || tokens::border())
        .child(group("Decks", move || library.decks.with(Vec::len)))
        .child(surface::separator())
        .child(group("Tags", move || library.tags.with(Vec::len)))
        .child(surface::separator())
        .child(group("Views", move || library.views.with(Vec::len)))
}

/// The app's root: the sidebar beside the work area, on the theme's background.
#[component]
pub fn App() -> impl IntoElement {
    crate::ui::tokens::install();
    provide_context(Library::sample());
    view()
        .flex_1()
        .flex_row()
        .bg(move || tokens::background())
        .child(sidebar())
        .child(view().flex_1().p_4().child(text("Pick a deck to study")))
}
