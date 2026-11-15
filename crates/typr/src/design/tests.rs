use super::*;
use rune::Runtime;

#[test]
fn the_spacing_scale_is_four_points_a_step() {
    assert_eq!(UNIT, 4.0);
    assert_eq!(space(0), 0.0);
    assert_eq!(space(1), 4.0);
    assert_eq!(space(3), 12.0);
    assert_eq!(space(6), 24.0);
    assert_eq!(size(4), space(4));
}

#[test]
fn the_off_scale_dimensions_are_the_two_named_ones() {
    assert_eq!(HAIRLINE, 1.0);
    assert_eq!(TIGHT, 2.0);
    assert_eq!(FOCUS_WIDTH, TIGHT);
}

#[test]
fn the_type_scale_leads_a_title_by_weight_not_size() {
    assert_eq!(type_body(), type_::BODY);
    assert_eq!(type_title(), type_::TITLE);
    assert_eq!(type_caption(), type_::MONO_CAPTION);
    assert_eq!(type_title().size, type_body().size);
}

#[test]
fn the_radii_come_from_the_kit() {
    assert_eq!(radius(), 6.0);
    assert_eq!(radius_small(), 4.0);
    assert_eq!(radius_large(), 8.0);
    assert!(radius_small() < radius() && radius() < radius_large());
}

#[test]
fn a_card_keeps_one_step_of_elevation() {
    assert!(!elevation().is_empty());
}

struct Held {
    focused: bool,
}

fn view(state: &Held, _: &mut rune::Cx<Held>) -> El {
    surface_card(
        vec![
            text("gato").font(type_body()).key("card-name"),
            text("1 review").font(type_caption()).key("card-meta"),
        ],
        state.focused,
    )
    .key("card")
}

fn draw(focused: bool) -> Runtime<Held> {
    let mut runtime = Runtime::new(
        Held { focused },
        view,
        (600.0, 400.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    runtime
}

#[test]
fn a_card_holds_its_children_inside_one_surface() {
    let runtime = draw(false);
    let elements = runtime.elements();
    let card = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("card"))
        .expect("the card");
    for key in ["card-name", "card-meta"] {
        let child = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("{key} is drawn"));
        let [x, y, w, h] = child.rect;
        let [cx, cy, cw, ch] = card.rect;
        assert!(
            x >= cx && y >= cy && x + w <= cx + cw + 0.5 && y + h <= cy + ch + 0.5,
            "{key} sits inside the card: {child:?} in {card:?}"
        );
    }
}

#[test]
fn the_focus_ring_takes_no_space() {
    let quiet = draw(false);
    let focused = draw(true);
    let bounds = |runtime: &Runtime<Held>| {
        runtime
            .elements()
            .iter()
            .find(|element| element.key.as_deref() == Some("card"))
            .map(|element| element.rect)
            .expect("the card")
    };
    assert_eq!(bounds(&quiet), bounds(&focused));
}
