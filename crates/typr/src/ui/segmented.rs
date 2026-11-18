//! A choice among a few named values: every value visible at once, the chosen one raised.
//!
//! The theme and a flag are this shape. A segment is a button named by its label, exposed as
//! selected when it is the one in force; the track behind them is the hover wash, so the chosen
//! segment reads as lifted off it by its card fill and border, not by colour alone.

use nsh_base::button as base_button;
use nsh_core::prelude::*;

use super::tokens;

/// `options` (a label and the value it stands for) as one control. `current` says which value is in
/// force and `on_change` hears the value of the segment pressed.
pub fn segmented<T: Copy + PartialEq + 'static>(
    options: Vec<(&'static str, T)>,
    current: impl Fn() -> T + Clone + 'static,
    on_change: impl Fn(T) + Clone + 'static,
) -> impl IntoElement {
    let mut track = view()
        .flex_row()
        .items_center()
        .gap_1()
        .p_1()
        .rounded(tokens::radius)
        .bg(tokens::hover);
    for (label, value) in options {
        let chosen = current.clone();
        let styled = current.clone();
        let colour = current.clone();
        let border = current.clone();
        let press = on_change.clone();
        track = track.child(
            base_button()
                .on_press(move || press(value))
                .selected(move || chosen() == value)
                .px_3()
                .py_1()
                .rounded(tokens::radius)
                .border_1()
                .border_color(move || {
                    if border() == value {
                        tokens::border()
                    } else {
                        tokens::hover()
                    }
                })
                .bg(move || {
                    if styled() == value {
                        tokens::card()
                    } else {
                        tokens::hover()
                    }
                })
                .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
                .child(text(label).text_sm().text_color(move || {
                    if colour() == value {
                        tokens::foreground()
                    } else {
                        tokens::muted_foreground()
                    }
                })),
        );
    }
    track
}
