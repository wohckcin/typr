//! The command palette: the current screen's commands, filtered as you type.

use rune::tok::type_;
use rune::{El, TextEdit, col, text, text_input};
use rune_kit::{Dialog, Kit};
use typr_core::keymap::{Command, Surface, bindings_of};
use typr_core::store::folded;

use super::Host;

static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

/// Whether the palette is open, what is typed into it, and which row is highlighted.
#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub filter: TextEdit,
    pub index: usize,
}

/// The commands a surface can run, in table order, without repeats.
pub fn commands(surface: Surface) -> Vec<Command> {
    let mut commands: Vec<Command> = Vec::new();
    for binding in bindings_of(surface) {
        if !commands.contains(&binding.command) {
            commands.push(binding.command);
        }
    }
    commands
}

/// Whether a command's label survives the filter.
pub fn shows(filter: &str, command: Command) -> bool {
    let needle = folded(filter.trim());
    needle.is_empty() || folded(command.label()).contains(&needle)
}

/// The commands the palette offers right now.
pub fn visible(surface: Surface, filter: &str) -> Vec<Command> {
    commands(surface)
        .into_iter()
        .filter(|command| shows(filter, *command))
        .collect()
}

/// Move the highlight down one row, stopping at the last.
pub fn move_down(palette: &mut Palette, surface: Surface) {
    let rows = visible(surface, &palette.filter.text).len();
    if rows > 0 {
        palette.index = (palette.index + 1).min(rows - 1);
    }
}

/// Move the highlight up one row, stopping at the first.
pub fn move_up(palette: &mut Palette) {
    palette.index = palette.index.saturating_sub(1);
}

/// Run the highlighted command, then close.
pub fn run<S: Host>(s: &mut S) -> bool {
    let surface = s.surface();
    let palette = s.palette();
    let rows = visible(surface, &palette.filter.text);
    let Some(command) = rows.get(palette.index).copied() else {
        return false;
    };
    let palette = s.palette_mut();
    palette.open = false;
    palette.filter = TextEdit::default();
    palette.index = 0;
    s.run_command(command)
}

/// Open the palette on a clean filter.
pub fn open(palette: &mut Palette) {
    palette.open = true;
    palette.filter = TextEdit::default();
    palette.index = 0;
}

/// Build the palette into an overlay layer. Nothing is added while it is closed.
pub fn build<S: Host>(s: &S, layer: &mut Vec<El>) {
    let surface = s.surface();
    let palette = s.palette();
    if !palette.open {
        return;
    }
    let rows = visible(surface, &palette.filter.text);
    let mut body = col().key("palette").w(460.0).gap(6.0).child(
        text_input(&palette.filter, type_::BODY)
            .key("palette-filter")
            .single_line()
            .w_full()
            .placeholder("Type a command", rune::tok::theme(true).text_secondary)
            .on_edit(|s: &mut S| &mut s.palette_mut().filter)
            .on_input_change(|s: &mut S, _| s.palette_mut().index = 0),
    );
    if rows.is_empty() {
        body = body.child(text("No command matches that.").font(type_::LABEL));
    }
    for (index, command) in rows.iter().copied().enumerate() {
        let mut button =
            rune_kit::Button::new(command.label()).variant(if index == palette.index {
                rune_kit::ButtonVariant::Neutral
            } else {
                rune_kit::ButtonVariant::Ghost
            });
        if let Some(chord) = super::keycap(s, command) {
            button = button.shortcut(chord);
        }
        body = body.child(
            button
                .on_press(move |s: &mut S| {
                    s.run_command(command);
                    let palette = s.palette_mut();
                    palette.open = false;
                    palette.filter = TextEdit::default();
                    palette.index = 0;
                })
                .build(&KIT)
                .key(format!("palette-{index}")),
        );
    }
    Dialog::new("palette", "Commands")
        .fit()
        .open(true)
        .on_close(|s: &mut S| s.palette_mut().open = false)
        .child(body)
        .build(&KIT, layer);
}
