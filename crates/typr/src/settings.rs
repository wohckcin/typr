//! The settings screen: every key the library holds, grouped the way the spec reads.

use rune::tok::type_;
use rune::{El, ScrollSpec, TextEdit, col, row, text, text_input};
use rune_kit::{Button, ButtonVariant, Kit, root};

use typr_core::store::{Group, SettingKey, SettingValue, Settings, StoreError, Theme};

/// What the settings screen needs from the app.
pub trait Host: 'static {
    /// The settings the library was last read with.
    fn settings(&self) -> &Settings;
    /// The text the screen is editing for `key`.
    fn setting_input(&self, key: SettingKey) -> &TextEdit;
    fn setting_input_mut(&mut self, key: SettingKey) -> &mut TextEdit;
    /// Send a validated value; a refusal comes back as a failure line, not as a panic.
    fn set_setting(&mut self, key: SettingKey, value: SettingValue);
    fn refuse(&mut self, error: &StoreError);
    fn leave_settings(&mut self);

    /// Read the field and send what it holds. A value the key does not accept is refused with the
    /// reason the store would give, and the stored value stays what it was.
    fn submit_setting(&mut self, key: SettingKey) {
        let text = self.setting_input(key).text.clone();
        match key.parse_value(&text) {
            Ok(value) => self.set_setting(key, value),
            Err(error) => self.refuse(&error),
        }
    }

    /// Put the documented default back in the field and store it.
    fn reset_setting(&mut self, key: SettingKey) {
        self.set_setting(key, key.default_value());
    }
}

/// The kit the selected theme draws with. Pure, so a view and a test read the same answer.
pub fn kit(settings: &Settings) -> Kit {
    Kit::neutral(settings.theme.is_dark())
}

/// The screen.
pub fn view<S: Host>(s: &S) -> El {
    let settings = s.settings();
    let kit = kit(settings);
    let t = kit.c;
    let mut sections = col().key("settings-sections").w_full().gap(28.0);
    for group in Group::ALL {
        sections = sections.child(section(s, group));
    }
    let body = col()
        .key("settings")
        .w_full()
        .h_full()
        .fill(t.page)
        .padding(24.0)
        .gap(16.0)
        .child(header(s))
        .child(sections.scroll(ScrollSpec {
            stick_bottom: false,
        }));
    root(body, Vec::new())
}

fn header<S: Host>(s: &S) -> El {
    let kit = kit(s.settings());
    row()
        .key("settings-header")
        .w_full()
        .center_y()
        .gap(12.0)
        .child(
            text("Settings")
                .key("settings-title")
                .font(type_::TITLE)
                .color(kit.c.text_base)
                .grow(1.0),
        )
        .child(
            Button::new("Done")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.leave_settings())
                .build(&kit)
                .key("settings-done"),
        )
}

fn section<S: Host>(s: &S, group: Group) -> El {
    let kit = kit(s.settings());
    let mut list = col()
        .key(format!("settings-{}", group.title().to_ascii_lowercase()))
        .w_full()
        .gap(14.0)
        .child(
            text(group.title())
                .font(type_::SECTION)
                .color(kit.c.text_muted),
        );
    for key in SettingKey::ALL
        .into_iter()
        .filter(|key| key.group() == group)
    {
        list = list.child(setting(s, key));
    }
    if group == Group::Review {
        list = list.child(
            text("A scheduler change takes effect on the next grade.")
                .font(type_::LABEL)
                .color(kit.c.text_muted),
        );
    }
    list
}

fn setting<S: Host>(s: &S, key: SettingKey) -> El {
    let kit = kit(s.settings());
    let t = kit.c;
    let mut line = row()
        .key(format!("setting-{}", key.as_str()))
        .w_full()
        .gap(16.0)
        .center_y()
        .child(
            col()
                .gap(2.0)
                .grow(1.0)
                .child(text(key.label()).font(type_::LABEL).color(t.text_base))
                .child(text(key.help()).font(type_::LABEL).color(t.text_muted)),
        );
    line = line.child(match key {
        SettingKey::Scheduler => toggle::<S>(s, key, scheduler_label(s.settings()).to_string()),
        SettingKey::ReverseDefault => toggle::<S>(
            s,
            key,
            if s.settings().reverse_default {
                "On"
            } else {
                "Off"
            }
            .to_string(),
        ),
        SettingKey::Theme => {
            let next = if s.settings().theme == Theme::Dark {
                Theme::Light
            } else {
                Theme::Dark
            };
            Button::new(format!("Use {}", next.label().to_ascii_lowercase()))
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.set_setting(key, SettingValue::Theme(next)))
                .build(&kit)
                .key("setting-theme-toggle")
        }
        _ => editor::<S>(s, key, &kit),
    });
    line
}

fn scheduler_label(settings: &Settings) -> &'static str {
    match settings.scheduler {
        typr_core::store::SchedulerKind::Fixed => "Fixed",
        typr_core::store::SchedulerKind::Fsrs => "FSRS",
    }
}

fn toggle<S: Host>(s: &S, key: SettingKey, label: String) -> El {
    let kit = kit(s.settings());
    let next = match (key, s.settings().value(key)) {
        (SettingKey::Scheduler, SettingValue::Scheduler(kind)) => {
            let next = match kind {
                typr_core::store::SchedulerKind::Fixed => typr_core::store::SchedulerKind::Fsrs,
                typr_core::store::SchedulerKind::Fsrs => typr_core::store::SchedulerKind::Fixed,
            };
            SettingValue::Scheduler(next)
        }
        (_, SettingValue::Flag(on)) => SettingValue::Flag(!on),
        _ => key.default_value(),
    };
    Button::new(label)
        .variant(ButtonVariant::Ghost)
        .on_press(move |s: &mut S| s.set_setting(key, next.clone()))
        .build(&kit)
        .key(format!("setting-{}-toggle", key.as_str()))
}

fn editor<S: Host>(s: &S, key: SettingKey, kit: &Kit) -> El {
    let width = match key {
        SettingKey::Weights => 320.0,
        _ => 96.0,
    };
    row()
        .gap(6.0)
        .center_y()
        .child(
            text_input(s.setting_input(key), type_::LABEL)
                .key(format!("setting-{}-input", key.as_str()))
                .w(width)
                .single_line()
                .on_edit(move |s: &mut S| s.setting_input_mut(key))
                .on_submit(move |s: &mut S, _| s.submit_setting(key)),
        )
        .child(
            Button::new("Set")
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.submit_setting(key))
                .build(kit)
                .key(format!("setting-{}-set", key.as_str())),
        )
        .child(
            Button::new("Reset")
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.reset_setting(key))
                .build(kit)
                .key(format!("setting-{}-reset", key.as_str())),
        )
}
