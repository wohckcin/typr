//! `typr` — a local-first flashcard app on Rune.

use rune::machine::Seq;
use rune::{AppSpec, Cx, El, col, text};
use typr::library::{self, Library};
use typr::screen::{Queue, Screen};

struct State {
    /// Drives `screen::step`; the library screen has no events of its own yet, so nothing reads
    /// it until the editor and session screens arrive.
    #[allow(dead_code)]
    seq: Seq,
    screen: Screen,
    library: Library,
}

impl Default for State {
    fn default() -> Self {
        Self { seq: Seq::default(), screen: Screen::Library, library: Library::default() }
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

fn view(s: &State, _: &mut Cx<State>) -> El {
    match s.screen {
        Screen::Library => library::view::<State>(s),
        other => {
            let label = match other {
                Screen::Editor { card: Some(_) } => "Editor",
                Screen::Editor { card: None } => "Editor (new card)",
                Screen::Session { queue: Queue::New } => "Session — new cards",
                Screen::Session { queue: Queue::Due } => "Session — due",
                Screen::Summary => "Summary",
                Screen::Library => unreachable!("handled above"),
            };
            col().w_full().h_full().padding(24.0).child(text(label))
        }
    }
}

fn main() {
    // The study day needs the machine's zone, and `time` only answers on the main thread, so read
    // it before any worker starts.
    let _ = typr::day::local_offset_ms();
    let scenes: &'static [(&'static str, fn(&mut State))] =
        &[("library-5000", |state: &mut State| state.library = Library::seeded(5000))];
    rune::run(AppSpec { scenes, ..AppSpec::new("typr", (960.0, 640.0), State::default, view) });
}
