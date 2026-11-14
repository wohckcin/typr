use super::{ViewMode, ViewMode::*};

#[test]
fn every_view_round_trips_through_its_name() {
    for view in ViewMode::ALL {
        assert_eq!(
            ViewMode::parse(view.as_str()),
            view,
            "{view:?} lost its name"
        );
    }
}

#[test]
fn an_unknown_name_reads_as_the_grid() {
    assert_eq!(
        ViewMode::parse("masonry"),
        Grid,
        "a deck must read as something"
    );
    assert_eq!(ViewMode::parse(""), Grid);
}

#[test]
fn the_grid_is_the_default() {
    assert_eq!(ViewMode::default(), Grid, "a deck with no view is a grid");
}

#[test]
fn the_switcher_lists_four_views_each_once() {
    assert_eq!(ViewMode::ALL.len(), 4);
    for (index, view) in ViewMode::ALL.iter().enumerate() {
        assert_eq!(
            ViewMode::ALL.iter().position(|other| other == view),
            Some(index),
            "{view:?} is listed twice"
        );
        assert!(!view.label().is_empty(), "{view:?} has no label");
    }
}
