//! `typr` — a local-first flashcard app on Rune.

use rune::machine::Seq;
use rune::{AppSpec, Cx, El, col, text};
use typr::editor::{self, Editor};
use typr::library::{self, Library};
use typr::screen::{Queue, Screen};

struct State {
    /// Drives `screen::step`; the library screen has no events of its own yet, so nothing reads
    /// it until the editor and session screens arrive.
    #[allow(dead_code)]
    seq: Seq,
    screen: Screen,
    library: Library,
    editor: Editor,
}

impl Default for State {
    fn default() -> Self {
        Self {
            seq: Seq::default(),
            screen: Screen::Library,
            library: Library::default(),
            editor: Editor::default(),
        }
    }
}

impl library::Host for State {
    fn library(&self) -> &Library {
        &self.library
    }

    fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }
}

impl editor::Host for State {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    match s.screen {
        Screen::Library => library::view::<State>(s),
        Screen::Editor { .. } => editor::view::<State>(s),
        other => {
            let label = match other {
                Screen::Session { queue: Queue::New } => "Session — new cards",
                Screen::Session { queue: Queue::Due } => "Session — due",
                Screen::Summary => "Summary",
                Screen::Library | Screen::Editor { .. } => unreachable!("handled above"),
            };
            col().w_full().h_full().padding(24.0).child(text(label))
        }
    }
}

fn main() {
    // The study day needs the machine's zone, and `time` only answers on the main thread, so read
    // it before any worker starts.
    let _ = typr::day::local_offset_ms();
    let scenes: &'static [(&'static str, fn(&mut State))] = &[
        ("library-5000", |state: &mut State| state.library = Library::seeded(5000)),
        ("editor", |state: &mut State| {
            state.screen = Screen::Editor { card: None };
            state.editor = Editor::new(None, "What is **mitosis**?\n---\nCell division.".to_string());
        }),
    ];
    rune::run(AppSpec { scenes, ..AppSpec::new("typr", (960.0, 640.0), State::default, view) });
}
