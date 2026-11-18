//! Overlays: a menu of commands, a popover of controls, a tooltip for an icon, a dialog for a
//! decision, and a toast for a result that needs no reply.

use std::time::Duration;

use nsh_base::{
    Anchor, ToastManager, ToastMotion, ToastOptions, ToastStackState, ToastTransitionStatus,
    dialog, menu, menu_item, popover, toast, toast_stack,
};
use nsh_core::{element::StyleRefinement, prelude::*, time::now};

use super::{motion, tokens};

/// The popup surface every anchored overlay shares.
fn popup() -> nsh_core::element::StyleRefinement {
    StyleRefinement::default()
        .min_w(px(192.0))
        .p_1()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .bg(tokens::popover())
}

/// A menu opened from `trigger`, with commands as its items.
pub fn command_menu(trigger: impl IntoElement, commands: Commands) -> impl IntoElement {
    let mut m = menu()
        .offset(px(4.0))
        .popup_style(|_| popup())
        .trigger(trigger);
    for (label, action) in commands {
        m = m.child(
            menu_item()
                .on_select(action)
                .px_3()
                .py_2()
                .rounded(tokens::radius)
                .hover(|s| s.bg(tokens::hover))
                .child(text(label).text_sm().text_color(tokens::foreground)),
        );
    }
    m
}

/// Controls shown beside their trigger without interrupting the task.
/// The rows a menu shows: a label and what it does.
pub type Commands = Vec<(&'static str, Box<dyn Fn()>)>;

pub fn anchored(trigger: impl IntoElement, content: impl IntoElement) -> impl IntoElement {
    popover().offset(px(6.0)).trigger(trigger).child(
        view()
            .p_3()
            .child(content)
            .bg(tokens::popover())
            .rounded(tokens::radius),
    )
}

/// A focused decision: shown while `open`, cancelled by Escape or a backdrop press. Its content
/// starts below the window's title bar, which the app draws under.
pub fn decision<V: IntoElement + 'static>(
    open: Signal<bool>,
    content: impl Fn() -> V + 'static,
) -> impl IntoElement {
    when(
        move || open.get(),
        move || {
            dialog()
                .on_cancel(move || open.set(false))
                .flex_1()
                .bg(tokens::background)
                .p_4()
                .pt_12()
                .child(content())
        },
    )
}

/// How long a result stays on screen before it leaves by itself.
pub const TOAST_TIMEOUT: Duration = Duration::from_secs(4);

type Shown = Vec<(u64, SharedString, ToastTransitionStatus)>;

/// The results a screen shows as toasts, oldest first. Their clock runs while `toasts(..)` is
/// mounted; a toast pushed meanwhile catches up when it next is.
#[derive(Clone, Copy)]
pub struct Toasts {
    manager: Stored<ToastManager<u64, SharedString>>,
    shown: Signal<Shown>,
    state: ToastStackState,
    next: Signal<u64>,
}

impl Default for Toasts {
    fn default() -> Self {
        Self::new()
    }
}

impl Toasts {
    pub fn new() -> Self {
        Self {
            manager: stored(ToastManager::new(ToastMotion::sonner())),
            shown: signal(Vec::new()),
            state: ToastStackState::new(),
            next: signal(0),
        }
    }

    /// Shows `text` as a new toast, which leaves after `TOAST_TIMEOUT`.
    pub fn push(&self, text: SharedString) {
        let id = self.next.get();
        self.next.set(id + 1);
        self.manager
            .with(|m| m.push(id, text, ToastOptions::timeout(Some(TOAST_TIMEOUT)), now()));
        self.sync();
    }

    /// Starts the exit of toast `id`, or of the newest one still showing.
    fn dismiss(&self, id: Option<u64>) {
        let now = now();
        let started = self.manager.with(|m| {
            let id = id.or_else(|| {
                m.iter()
                    .rev()
                    .find(|(_, _, s)| *s != ToastTransitionStatus::Ending)
                    .map(|(id, _, _)| *id)
            });
            id.is_some_and(|id| m.dismiss(&id, now))
        });
        if started {
            self.sync();
        }
    }

    fn sync(&self) {
        self.shown.set(
            self.manager
                .with(|m| m.visible(3).map(|(id, t, s)| (*id, t.clone(), s)).collect()),
        );
    }
}

/// The toasts of `toasts`, each a popup-style line of text, floating at the bottom centre of the
/// window: the stack takes no room where it is drawn, so the screen's own layout is untouched, and
/// the overlay takes no press, which leaves what it floats over usable.
pub fn toasts(toasts: &Toasts) -> impl IntoElement {
    let toasts = *toasts;
    // The lifecycle clock, owned by this element so it stops when the screen leaves; paused while
    // the stack is expanded.
    set_interval(Duration::from_millis(50), move || {
        if toasts
            .manager
            .with(|m| m.advance(now(), toasts.state.is_expanded()).changed)
        {
            toasts.sync();
        }
    });
    overlay()
        .absolute()
        .inset_0()
        .items_center()
        .justify_end()
        .p_6()
        .pointer_events(PointerEvents::None)
        .child(
            toast_stack()
                .placement(Anchor::BottomCenter)
                .state(toasts.state)
                .on_cancel(move || toasts.dismiss(None))
                .w(px(340.0))
                .child(each(
                    toasts.shown,
                    |(id, _, _)| *id,
                    move |item, _| {
                        toast()
                            .transition_status(move || item.get().2)
                            .accessibility_label(move || Some(item.get().1))
                            .on_dismiss_request(move || toasts.dismiss(Some(item.get().0)))
                            // One beat inside the manager's 200 ms exit, so the fade ends before removal.
                            .transition_all()
                            .duration(motion::FAST)
                            .starting_class("opacity-0 translate-y-2")
                            .ending_class("opacity-0 scale-95")
                            .p_3()
                            .rounded(tokens::radius)
                            .border_1()
                            .border_color(tokens::border)
                            .bg(tokens::popover())
                            .child(
                                text(move || item.get().1)
                                    .text_sm()
                                    .text_color(tokens::foreground),
                            )
                    },
                )),
        )
}
