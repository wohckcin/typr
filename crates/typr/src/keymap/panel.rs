//! The shortcut panel: every binding the current screen can run, filtered as you type.

use rune::tok::type_;
use rune::{El, TextEdit, col, row, text, text_input};
use rune_kit::{Dialog, Kit};
use typr_core::keymap::{Binding, Surface, bindings_of};
use typr_core::store::folded;

use super::Host;

static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

/// Whether the panel is open, and what is typed into its filter.
#[derive(Default)]
pub struct Panel {
    pub open: bool,
    pub filter: TextEdit,
}

/// Whether a binding survives the filter, matching either its chord or its label.
pub fn shows(filter: &str, binding: &Binding) -> bool {
    let needle = folded(filter.trim());
    needle.is_empty()
        || folded(&binding.chord.to_string()).contains(&needle)
        || folded(binding.command.label()).contains(&needle)
}

/// The rows a surface shows under one filter.
pub fn rows<'a>(surface: Surface, filter: &'a str) -> impl Iterator<Item = &'static Binding> + 'a {
    bindings_of(surface).filter(move |binding| shows(filter, binding))
}

/// Build the panel into an overlay layer. Nothing is added while it is closed.
pub fn build<S: Host>(s: &S, layer: &mut Vec<El>) {
    let panel = s.panel();
    if !panel.open {
        return;
    }
    let mut body = col().key("shortcuts").w(420.0).gap(10.0).child(
        text_input(&panel.filter, type_::LABEL)
            .key("shortcuts-filter")
            .single_line()
            .w_full()
            .placeholder(
                "Filter the shortcuts",
                rune::tok::theme(true).text_secondary,
            )
            .on_edit(|s: &mut S| &mut s.panel_mut().filter),
    );
    let mut any = false;
    for surface in [
        Surface::Library,
        Surface::Editor,
        Surface::Session,
        Surface::Summary,
    ] {
        let mut group = col().key("group").gap(4.0);
        let mut shown = 0;
        for binding in rows(surface, &panel.filter.text) {
            shown += 1;
            any = true;
            group = group.child(
                row()
                    .key("shortcut")
                    .gap(10.0)
                    .center_y()
                    .child(super::kbd(binding.chord))
                    .child(text(binding.command.label()).font(type_::LABEL)),
            );
        }
        if shown > 0 {
            body = body
                .child(
                    text(surface.name())
                        .font(type_::LABEL)
                        .color(rune::tok::theme(true).text_secondary),
                )
                .child(group);
        }
    }
    if !any {
        body = body.child(text("Nothing matches that filter.").font(type_::LABEL));
    }
    Dialog::new("shortcuts", "Keyboard shortcuts")
        .fit()
        .open(true)
        .on_close(|s: &mut S| s.panel_mut().open = false)
        .child(body)
        .build(&KIT, layer);
}
