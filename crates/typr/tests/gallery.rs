//! The ui gallery: every component the app's screens are built from, in one place, pressed by
//! the user's steps. A component that breaks here breaks every screen that uses it.

use nsh_base::SelectItem;
use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::{
    button::{Variant, busy_button, button},
    choice::{Choice, TabSelect, choose, list_row, tab_row},
    input::{FieldInput, field},
    overlay::{Toasts, anchored, command_menu, decision, toasts},
    toggle::{CheckChange, ToggleChange, check, toggle},
};

const TABS: &[&str] = &["Notes", "History"];

#[component]
fn Gallery() -> impl IntoElement {
    typr::ui::tokens::install();
    let tab = signal(0_usize);
    let row = signal(0_usize);
    let open = signal(false);
    let results = Toasts::new();
    let renamed = signal(false);
    let chosen = signal(None::<SharedString>);
    let draft = signal(String::new());
    let ticked = signal(false);
    let sound = signal(false);
    let saving = signal(false);
    let commands: typr::ui::overlay::Commands =
        vec![("Rename", Box::new(move || renamed.set(true)))];
    view()
        .gap_3()
        .p_4()
        .child(
            field()
                .placeholder("Search")
                .value(move || SharedString::from(draft.get()))
                .on_change(move |input: &FieldInput| draft.set(input.text.clone())),
        )
        .child(
            check()
                .label("Remember")
                .checked(move || ticked.get())
                .on_change(move |next: &CheckChange| ticked.set(next.checked)),
        )
        .child(
            toggle()
                .label("Sound")
                .on(move || sound.get())
                .on_change(move |next: &ToggleChange| sound.set(next.on)),
        )
        .child(text(move || {
            format!(
                "typed={} ticked={} sound={}",
                draft.with(|d| d.len()),
                ticked.get(),
                sound.get()
            )
        }))
        .child(
            tab_row()
                .labels(
                    TABS.iter()
                        .map(|label| SharedString::from(*label))
                        .collect::<Vec<_>>(),
                )
                .selected(move || tab.get())
                .on_select(move |select: &TabSelect| tab.set(select.index)),
        )
        .child(text(move || format!("tab={}", tab.get())))
        .child(
            list_row()
                .title("First row")
                .selected(move || row.get() == 0)
                .on_press(move || row.set(0)),
        )
        .child(
            list_row()
                .title("Second row")
                .selected(move || row.get() == 1)
                .on_press(move || row.set(1)),
        )
        .child(text(move || format!("row={}", row.get())))
        .child(
            button()
                .variant(Variant::Primary)
                .label("Open")
                .disabled(|| false)
                .on_press(move || open.set(true)),
        )
        .child(decision(open, move || {
            view().gap_2().child(text("Confirm the change?")).child(
                button()
                    .variant(Variant::Default)
                    .label("Close")
                    .disabled(|| false)
                    .on_press(move || open.set(false)),
            )
        }))
        .child(
            button()
                .variant(Variant::Destructive)
                .label("Notify")
                .disabled(|| false)
                .on_press(move || results.push("Saved".into())),
        )
        .child(toasts(&results))
        .child(command_menu(
            button()
                .variant(Variant::Ghost)
                .label("More")
                .disabled(|| false)
                .on_press(|| {}),
            commands,
        ))
        .child(anchored(
            button()
                .variant(Variant::Ghost)
                .label("Details")
                .disabled(|| false)
                .on_press(|| {}),
            text("Shipped"),
        ))
        .child(
            choose()
                .options(vec![
                    SelectItem::new("apple", "Apple"),
                    SelectItem::new("kiwi", "Kiwi"),
                ])
                .chosen(move || chosen.get())
                .on_change(move |next: &Choice| chosen.set(next.value.clone())),
        )
        .child(text(move || format!("renamed={}", renamed.get())))
        .child(
            button()
                .variant(Variant::Default)
                .label("Locked")
                .disabled(|| true)
                .on_press(|| {}),
        )
        .child(busy_button(
            Variant::Primary,
            "Save",
            move || saving.get(),
            move || saving.set(true),
        ))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Gallery>())).unwrap()
}

#[test]
fn choosing_a_tab_selects_its_section() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:/tab=0/")?;
    app.press("role:tab[History]")?;
    app.wait_for("text:/tab=1/")?;
    Ok(())
}

#[test]
fn pressing_a_row_selects_it() -> Result<(), String> {
    let mut app = start();
    app.wait_for("First row")?;
    app.press("First row")?;
    app.press("Second row")?;
    app.wait_for("text:/row=1/")?;
    Ok(())
}

#[test]
fn a_decision_opens_and_closes() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Open]")?;
    app.press("role:button[Open]")?;
    app.wait_for("Confirm the change?")?;
    app.press("role:button[Close]")?;
    app.wait_for_gone("Confirm the change?", std::time::Duration::from_secs(2))?;
    Ok(())
}

#[test]
fn a_notification_shows_its_message() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Notify]")?;
    app.press("role:button[Notify]")?;
    app.wait_for("Saved")?;
    Ok(())
}

#[test]
fn a_menu_runs_its_command() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[More]")?;
    app.press("role:button[More]")?;
    app.wait_for("Rename")?;
    app.press("Rename")?;
    app.wait_for("text:/renamed=true/")?;
    Ok(())
}

#[test]
fn a_popover_shows_its_content_when_its_trigger_is_pressed() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Details]")?;
    app.press("role:button[Details]")?;
    app.wait_for("Shipped")?;
    Ok(())
}

#[test]
fn a_select_shows_its_placeholder_until_a_value_is_chosen() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Choose")?;
    Ok(())
}

#[test]
fn a_disabled_button_is_not_enabled() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Locked]")?;
    assert!(!app.screen().enabled("role:button[Locked]")?);
    Ok(())
}

#[test]
fn a_busy_button_says_it_is_working_and_cannot_be_pressed() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Save]")?;
    app.press("role:button[Save]")?;
    app.wait_for("role:button[Working…]")?;
    assert!(!app.screen().enabled("role:button[Working…]")?);
    Ok(())
}
