//! The diagram pane: the picture, a cover per rectangle, the drag that draws one, and the geometry
//! that turns a drag into fractions of the picture.

use nsh_core::element::JsObject;
use nsh_core::prelude::*;
use nsh_core::{TouchEvent, TouchPhase};
use typr_core::store::{MaskRect, MaskRow};

use super::{EditorEvent, GroupMerge, MaskAdded, MaskGroup};
use crate::ui::{
    button::{Variant, button},
    prose::prose,
    tokens,
};

/// A rectangle as fractions of the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn of(rect: MaskRect) -> Rect {
        Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }

    fn rect(self) -> MaskRect {
        MaskRect::new(self.x, self.y, self.w, self.h)
    }
}

/// One cover on a diagram's picture: the group it belongs to, and its rectangle as fractions of
/// the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct Cover {
    pub group: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Cover {
    /// A store row as the pane draws it.
    pub fn of(row: &MaskRow) -> Cover {
        Cover {
            group: row.group,
            x: row.rect.x,
            y: row.rect.y,
            w: row.rect.w,
            h: row.rect.h,
        }
    }

    /// The rectangle the store holds.
    fn rect(&self) -> MaskRect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
        .rect()
    }
}

/// A diagram card's picture, its caption, and the rectangles drawn on it.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct Diagram {
    pub image: String,
    pub caption: String,
    pub rows: Vec<Cover>,
}

/// The diagram pane: the picture, a cover per rectangle labelled with its group, a drag across it
/// that adds a rectangle to the last group, and actions that merge the picked group into one
/// rectangle or remove the picked group.
pub fn diagram_pane(diagram: Diagram, cx: Scope<EditorEvent>) -> impl IntoElement {
    let Diagram {
        image,
        caption,
        rows,
    } = diagram;
    // How large the picture is drawn, and where a finger went down on it.
    let size = signal((300.0_f32, 200.0_f32));
    let from = signal(None::<(f32, f32)>);
    let to = signal(None::<(f32, f32)>);
    let picked = signal(None::<usize>);
    // The groups in the order they first appear, so the strip is stable as rectangles are added.
    let mut groups: Vec<u32> = Vec::new();
    for row in &rows {
        if !groups.contains(&row.group) {
            groups.push(row.group);
        }
    }
    // A drag joins the last group, so repeated drags build one answer.
    let group = groups.last().copied().unwrap_or(1);
    let layer_rows = rows.clone();
    let merge_rows = rows.clone();
    let remove_rows = rows;
    let mut strip = view().flex_row().gap_2();
    for (index, row) in layer_rows.clone().into_iter().enumerate() {
        strip = strip.child(
            button()
                .variant(Variant::Default)
                .label(format!("group {}", row.group))
                .disabled(|| false)
                .on_press(move || picked.set(Some(index))),
        );
    }
    view()
        .flex_1()
        .gap_2()
        .claim_touches(TouchClaim::on_move())
        // A drag across the picture runs from where the finger went down to where it lifted.
        .on_touch(move |event: &TouchEvent| {
            let (width, height) = size.get_untracked();
            // The picture takes the touches itself (`PointerEvents::BoxOnly`), so a finger's
            // position is already relative to it.
            let position = event
                .points
                .first()
                .map(|point| (point.location.x.0, point.location.y.0));
            match event.phase {
                TouchPhase::Start => {
                    if let Some(at) = position {
                        from.set(Some(at));
                        to.set(Some(at));
                    }
                }
                TouchPhase::Move => {
                    if let Some(at) = position {
                        to.set(Some(at));
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => {
                    let started = from.get_untracked();
                    let ended = position.or_else(|| to.get_untracked());
                    from.set(None);
                    to.set(None);
                    let (Some(started), Some(ended), TouchPhase::End) =
                        (started, ended, &event.phase)
                    else {
                        return;
                    };
                    let Some(rect) = rect_from_corners((width, height), started, ended) else {
                        return;
                    };
                    cx.emit(EditorEvent::AddMask(MaskAdded {
                        mask: Cover {
                            group,
                            x: rect.x,
                            y: rect.y,
                            w: rect.w,
                            h: rect.h,
                        },
                    }));
                }
            }
        })
        .child(
            view()
                .flex_1()
                .overflow_hidden()
                .accessible(true)
                .role(AccessibilityRole::Image)
                .label("the picture")
                .pointer_events(PointerEvents::BoxOnly)
                .on_layout(move |bounds: &Bounds<Points>| {
                    size.set((bounds.size.width.0, bounds.size.height.0));
                })
                .child(
                    text("diagram")
                        .text_xs()
                        .text_color(tokens::muted_foreground),
                )
                .child(prose().source(move || SharedString::from(format!("![]({image})"))))
                .child(dynamic(move || {
                    let dims = size.get();
                    let selected = picked.get();
                    let mut layer = view().absolute().inset(tokens::flush());
                    for (index, row) in layer_rows.clone().into_iter().enumerate() {
                        let (left, top, width, height) = cover_box(dims, row.rect());
                        let fill = move || match selected == Some(index) {
                            true => tokens::selection(),
                            false => tokens::selection().opacity(0.25),
                        };
                        layer = layer.child(
                            view()
                                .absolute()
                                .left(px(left))
                                .top(px(top))
                                .w(px(width))
                                .h(px(height))
                                .bg(fill)
                                .border_1()
                                .border_color(tokens::selection)
                                .accessible(true)
                                .label(format!("rectangle in group {}", row.group))
                                .child(text(format!("group {}", row.group)).text_xs()),
                        );
                    }
                    layer.into_element()
                })),
        )
        .child(text(caption).text_sm())
        .child(strip)
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(
                    button()
                        .variant(Variant::Default)
                        .label("Merge group")
                        .disabled(|| false)
                        .on_press(move || {
                            let Some(index) = picked.get_untracked() else {
                                return;
                            };
                            let group = merge_rows[index].group;
                            let rects: Vec<Rect> = merge_rows
                                .iter()
                                .filter(|row| row.group == group)
                                .map(|row| Rect::of(row.rect()))
                                .collect();
                            cx.emit(EditorEvent::MergeMasks(GroupMerge { group, rects }));
                        }),
                )
                .child(
                    button()
                        .variant(Variant::Default)
                        .label("Remove group")
                        .disabled(|| false)
                        .on_press(move || {
                            let Some(index) = picked.get_untracked() else {
                                return;
                            };
                            cx.emit(EditorEvent::RemoveMaskGroup(MaskGroup {
                                group: remove_rows[index].group,
                            }));
                        }),
                ),
        )
}

/// The rectangle between two points on the picture, as fractions of the pane `size` points wide.
///
/// `None` when either side is under one percent — the spec's floor — so a click or a twitch adds
/// nothing, and when the pane has no size yet.
pub fn rect_from_corners(size: (f32, f32), from: (f32, f32), to: (f32, f32)) -> Option<MaskRect> {
    let (pane_w, pane_h) = size;
    if pane_w <= 0.0 || pane_h <= 0.0 {
        return None;
    }
    let fraction = |value: f32, span: f32| (value / span).clamp(0.0, 1.0);
    let x0 = fraction(from.0.min(to.0), pane_w);
    let x1 = fraction(from.0.max(to.0), pane_w);
    let y0 = fraction(from.1.min(to.1), pane_h);
    let y1 = fraction(from.1.max(to.1), pane_h);
    // The far edge is clamped twice over: a drag that runs off the picture ends at its edge, and
    // the sum stays within one without a float-error overshoot the store would refuse.
    let rect = MaskRect::new(x0, y0, (x1 - x0).min(1.0 - x0), (y1 - y0).min(1.0 - y0));
    if rect.w < 0.01 || rect.h < 0.01 || !rect.inside_the_image() {
        return None;
    }
    Some(rect)
}

/// Where a cover sits inside a pane `size` points wide, in points: `(left, top, width, height)`.
pub fn cover_box(size: (f32, f32), rect: typr_core::store::MaskRect) -> (f32, f32, f32, f32) {
    (
        rect.x * size.0,
        rect.y * size.1,
        rect.w * size.0,
        rect.h * size.1,
    )
}
