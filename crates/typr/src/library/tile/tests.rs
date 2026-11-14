use super::{State, chips, first_side, hidden_sides, state, words};
use crate::library::Library;
use typr_core::store::{CardSummary, Id};

const NOON: i64 = 1_700_000_000_000;
const DAY: i64 = 24 * 60 * 60 * 1000;

fn card(sides: u32, markdown: &str) -> CardSummary {
    CardSummary {
        id: 1,
        deck: 1,
        kind: typr_core::store::CardKind::Text,
        image: None,
        caption: None,
        masks: Vec::new(),
        sides,
        tags: Vec::new(),
        archived: false,
        reverse: None,
        trashed_at: None,
        updated_at: 0,
        preview: String::new(),
        markdown: markdown.to_string(),
        search_text: String::new(),
        prompts: u32::from(sides >= 2),
        unscheduled: 0,
        due_at: None,
    }
}

#[test]
fn a_tile_draws_the_first_side_only() {
    let library = Library::seeded(1);
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| {
            let card = s.cards().first().expect("one card").clone();
            super::tile::<Library>(s, &card, 245.0, NOON)
        },
        (400.0, 400.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.1);
    let strings: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|el| el.text)
        .collect();
    assert!(
        strings.iter().any(|text| text.contains("Card 1")),
        "the first side is drawn: {strings:?}"
    );
}

#[test]
fn hidden_sides_count_every_side_but_the_one_shown() {
    assert_eq!(hidden_sides(&card(1, "note")), 0);
    assert_eq!(hidden_sides(&card(2, "a\n---\nb")), 1);
    assert_eq!(hidden_sides(&card(4, "a\n---\nb\n---\nc\n---\nd")), 3);
}

#[test]
fn a_tile_states_the_sides_it_hides() {
    let two = card(2, "a\n---\nb");
    assert_eq!(hidden_sides(&two), 1);
    let mut three = card(3, "a\n---\nb\n---\nc");
    three.sides = 3;
    assert_eq!(hidden_sides(&three), 2, "three sides hide two");
}

#[test]
fn a_note_carries_no_hidden_side_badge() {
    let mut note = card(1, "just one side");
    note.prompts = 0;
    assert_eq!(hidden_sides(&note), 0);
    assert_eq!(state(&note, NOON), State::Note);
}

#[test]
fn a_tile_states_where_the_card_stands() {
    let mut new = card(2, "a\n---\nb");
    new.prompts = 1;
    new.unscheduled = 1;
    assert_eq!(state(&new, NOON), State::New);
    assert_eq!(words(State::New), ("New".to_string(), "NEW"));

    let mut due = new.clone();
    due.due_at = Some(NOON - 1);
    assert_eq!(state(&due, NOON), State::Due);
    assert_eq!(words(State::Due), ("Due".to_string(), "DUE"));

    let mut later = new.clone();
    later.due_at = Some(NOON + 3 * DAY);
    assert_eq!(state(&later, NOON), State::Interval(3));
    assert_eq!(words(State::Interval(3)).0, "3 days");

    let mut hour = new.clone();
    hour.due_at = Some(NOON + 60 * 60 * 1000);
    assert_eq!(
        state(&hour, NOON),
        State::Interval(1),
        "an hour away is one day, not zero"
    );
    assert_eq!(words(State::Interval(1)).0, "1 day", "the singular is used");
}

#[test]
fn the_chips_count_what_they_drop() {
    let tags: Vec<String> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|t| t.to_string())
        .collect();
    let (shown, dropped) = chips(&tags, 1000.0);
    assert_eq!(shown.len(), 5, "all five fit a wide tile");
    assert_eq!(dropped, 0);

    let (shown, dropped) = chips(&tags, 40.0);
    assert_eq!(shown.len(), 1, "only the first chip fits a narrow strip");
    assert_eq!(dropped, 4, "and four are counted");
}

#[test]
fn a_card_id_is_stable_for_the_parse_key() {
    let one = card(2, "a\n---\nb");
    let mut other = one.clone();
    other.updated_at = 7;
    assert_ne!(super::side_key(&one), super::side_key(&other));
    assert_eq!(super::side_key(&one), super::side_key(&one.clone()));
    let _: Id = one.id;
}

/// A state that draws one tile, so a fn-pointer view can reach the card and the width.
struct Probe {
    library: Library,
    card: CardSummary,
    width: f32,
}

impl Probe {
    fn new(card: CardSummary, width: f32) -> Probe {
        Probe {
            library: Library::seeded(1),
            card,
            width,
        }
    }

    fn measure(self) -> ([f32; 4], Vec<String>) {
        fn draw(s: &Probe, _: &mut rune::Cx<Probe>) -> rune::El {
            rune::col()
                .w_full()
                .items_start()
                .child(super::tile::<Probe>(s, &s.card, s.width, NOON))
        }
        let mut runtime =
            rune::Runtime::new(self, draw, (1200.0, 600.0), rune::Env::new(true, 1.0));
        runtime.frame(0.0);
        runtime.frame(0.1);
        let elements = runtime.elements();
        let rect = elements
            .iter()
            .find(|el| el.key.as_deref() == Some("tile-1"))
            .expect("the tile is drawn")
            .rect;
        let strings = elements
            .into_iter()
            .filter_map(|el| el.text)
            .collect::<Vec<_>>();
        (rect, strings)
    }
}

impl crate::library::Host for Probe {
    fn library(&self) -> &Library {
        &self.library
    }

    fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }
}

#[test]
fn a_tile_is_exactly_the_width_it_is_given() {
    for width in [227.0, 245.0, 287.0] {
        let (rect, _) = Probe::new(card(2, "a\n---\nb"), width).measure();
        assert!(
            (rect[2] - width).abs() < 0.5,
            "a {width} point share draws a {} point tile",
            rect[2]
        );
    }
}

#[test]
fn a_tile_states_its_footer_in_the_right_case() {
    let mut due = card(2, "a\n---\nb");
    due.prompts = 1;
    due.unscheduled = 1;
    let (_, strings) = Probe::new(due, 245.0).measure();
    assert!(
        strings.iter().any(|text| text == "New"),
        "the footer says New in sentence case: {strings:?}"
    );
    assert!(
        strings.iter().any(|text| text == "NEW"),
        "and its kicker is uppercase: {strings:?}"
    );

    let mut hidden = card(3, "a\n---\nb\n---\nc");
    hidden.sides = 3;
    let (_, strings) = Probe::new(hidden, 245.0).measure();
    assert!(
        strings.iter().any(|text| text == "2 HIDDEN SIDES"),
        "the strip says how many sides are hidden: {strings:?}"
    );

    let mut one = card(2, "a\n---\nb");
    one.due_at = Some(NOON + DAY);
    let (_, strings) = Probe::new(one, 245.0).measure();
    assert!(
        strings.iter().any(|text| text == "1 day"),
        "an interval reads in the singular: {strings:?}"
    );
}

#[test]
fn a_tagged_tile_is_not_shorter_than_an_untagged_one() {
    let mut tagged = card(2, "a\n---\nb");
    tagged.tags = vec!["Grammar".to_string(), "Verbs".to_string()];
    let (tall, _) = Probe::new(tagged, 245.0).measure();
    let (short, _) = Probe::new(card(2, "a\n---\nb"), 245.0).measure();
    assert!(
        tall[3] >= short[3],
        "a tile with chips is not shorter than one without: {} against {}",
        tall[3],
        short[3]
    );
}

#[test]
fn a_tile_shows_the_masked_question_of_a_card_that_hides_its_answer() {
    let shown = first_side(&card(1, "The {{gland}} is small."));

    assert_eq!(
        shown, "The [ … ] is small.",
        "the tile asks the question, not the answer"
    );
    assert!(!shown.contains("gland"), "the answer stays off the grid");
}

#[test]
fn a_tile_shows_the_first_side_of_a_card_with_sides() {
    assert_eq!(first_side(&card(2, "front\n---\nback")), "front");
}

#[test]
fn a_tile_hides_every_answer_a_hidden_text_card_holds() {
    let library = Library::seeded(1);
    let mut card = library.cards().first().expect("one card").clone();
    card.markdown = "Our lungs pull {{2::oxygen}} from the {{1::air}}.".to_string();
    let (_, strings) = Probe::new(card, 245.0).measure();
    let drawn = strings.join(" ");
    assert!(
        drawn.contains("[1 … ]"),
        "the first group is named: {strings:?}"
    );
    assert!(
        drawn.contains("[2 … ]"),
        "the second group is named: {strings:?}"
    );
    assert!(
        !drawn.contains("oxygen"),
        "no answer is readable: {strings:?}"
    );
    assert!(
        !drawn.contains("air."),
        "no answer is readable: {strings:?}"
    );
}

#[test]
fn a_card_asked_both_ways_says_so_on_its_tile() {
    let mut both = card(2, "front\n---\nback");
    both.reverse = Some(true);
    let (_, drawn) = Probe {
        library: Library::seeded(1),
        card: both,
        width: 245.0,
    }
    .measure();
    assert!(
        drawn.iter().any(|line| line == "BOTH WAYS"),
        "the footer names the direction where the other states are named: {drawn:?}"
    );

    let (_, forward) = Probe {
        library: Library::seeded(1),
        card: card(2, "front\n---\nback"),
        width: 245.0,
    }
    .measure();
    assert!(
        forward.iter().all(|line| line != "BOTH WAYS"),
        "and a card that never set it says nothing: {forward:?}"
    );
}
