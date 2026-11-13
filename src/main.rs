//! `typr` — a local-first flashcard app on Rune.

use rune::machine::Seq;
use rune::{AppSpec, Cx, El, col, text};
use typr::screen::{Queue, Screen};

struct State {
    seq: Seq,
    screen: Screen,
}

impl Default for State {
    fn default() -> Self {
        Self { seq: Seq::default(), screen: Screen::Library }
    }
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let label = match s.screen {
        Screen::Library => "Library",
        Screen::Editor { card: Some(_) } => "Editor",
        Screen::Editor { card: None } => "Editor (new card)",
        Screen::Session { queue: Queue::New } => "Session — new cards",
        Screen::Session { queue: Queue::Due } => "Session — due",
        Screen::Summary => "Summary",
    };
    col().w_full().h_full().padding(24.0).child(text(label))
}

fn main() {
    // The study day needs the machine's zone, and `time` only answers on the main thread, so read
    // it before any worker starts.
    let _ = typr::day::local_offset_ms();
    rune::run(AppSpec::new("typr", (960.0, 640.0), State::default, view));
}
