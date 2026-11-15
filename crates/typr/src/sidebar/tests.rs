use super::*;
use rune::Env;

/// The heights and offsets of the three rows drawn below, keyed by row.
fn drawn() -> Vec<(String, [f32; 4], [f32; 4])> {
    let mut runtime = rune::Runtime::new(
        (),
        |_: &(), _: &mut rune::Cx<()>| {
            col()
                .px(INSET)
                .w(WIDTH)
                .child(row::<()>(
                    &Row::new("row-all", "All cards"),
                    |_: &mut ()| {},
                ))
                .child(row::<()>(
                    &Row::new("row-spanish", "Spanish").selected(),
                    |_: &mut ()| {},
                ))
                .child(row::<()>(
                    &Row::new("row-import", "Import").disabled(),
                    |_: &mut ()| {},
                ))
        },
        (WIDTH + 40.0, 200.0),
        Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    let elements = runtime.elements();
    let mut rows = Vec::new();
    for name in ["row-all", "row-spanish", "row-import"] {
        let row = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(name));
        let wanted = format!("{name}-label");
        let label = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(wanted.as_str()));
        match (row, label) {
            (Some(row), Some(label)) => rows.push((name.to_string(), row.rect, label.rect)),
            _ => panic!("{name} was not drawn"),
        }
    }
    rows
}

#[test]
fn rows_share_one_label_offset_and_height() {
    for (name, row, label) in drawn() {
        assert!(
            (row[3] - ROW_HEIGHT).abs() < 0.5,
            "{name} is {} points tall",
            row[3]
        );
        assert!(
            (label[0] - row[0] - LABEL_X).abs() < 0.5,
            "{name} starts its label {} points in, not {LABEL_X}",
            label[0] - row[0]
        );
    }
}

#[test]
fn every_row_starts_at_the_same_inset() {
    let offsets: Vec<f32> = drawn().iter().map(|(_, row, _)| row[0]).collect();
    assert!(
        offsets
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.5),
        "rows start at {offsets:?}"
    );
}
