//! Every construct in the one list, drawn once: a test per row with the source it draws, plus the two spec
//! scenarios about text that must stay as written.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::markdown_ext::supported::SUPPORTED;

/// Draws `source` through the app's prose, then checks the screen for `shown` and never for `hidden`.
macro_rules! draws {
    ($name:ident, $source:expr, $shown:expr, $hidden:expr) => {
        #[test]
        fn $name() -> Result<(), String> {
            #[component]
            fn Probe() -> impl IntoElement {
                typr::ui::tokens::install();
                typr::ui::prose::prose().source($source)
            }
            let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
            app.wait_for(&format!("text:/{}/", $shown))?;
            let joined = app.screen().texts().join(" ");
            assert!(
                !joined.contains($hidden),
                "{:?} still shows {:?}: {:?}",
                $source,
                $hidden,
                app.screen().texts()
            );
            Ok(())
        }
    };
}

// The constructs the renderer draws itself.

draws!(strong_is_drawn, "**b**", "b", "**");
draws!(emphasis_is_drawn, "*i*", "i", "*");
draws!(strikethrough_is_drawn, "~~s~~", "s", "~~");
draws!(inline_code_is_drawn, "`c`", "c", "`");
draws!(a_code_fence_is_drawn, "```\nx\n```", "x", "```");
draws!(
    a_table_is_drawn,
    "| a | b |\n|---|---|\n| 1 | 2 |",
    "a",
    "|---|"
);
draws!(a_task_list_is_drawn, "- [x] done", "done", "[x]");
draws!(a_link_is_drawn, "[t](https://e.com)", "t", "](");
#[test]
fn an_image_is_drawn() -> Result<(), String> {
    #[component]
    fn Probe() -> impl IntoElement {
        typr::ui::tokens::install();
        typr::ui::prose::prose().source("before ![alt](x.png) after")
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/before/")?;
    assert!(
        !app.screen().texts().join(" ").contains("!["),
        "the marker is gone"
    );
    Ok(())
}

draws!(a_heading_is_drawn, "# h", "h", "#");
draws!(a_list_is_drawn, "- one\n- two", "one", "- one");
draws!(a_quote_is_drawn, "> q", "q", "> q");
draws!(subscript_is_drawn, "H~2~0", "H20", "~2~");
draws!(inline_math_is_drawn, "a $E=mc^2$", "a", "$");
#[test]
fn display_math_is_drawn() -> Result<(), String> {
    #[component]
    fn Probe() -> impl IntoElement {
        typr::ui::tokens::install();
        typr::ui::prose::prose().source("above\n\n$$\\int x$$")
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/above/")?;
    assert!(
        !app.screen().texts().join(" ").contains("$$"),
        "no dollar markers"
    );
    Ok(())
}

// The three this change adds.

draws!(highlight_is_drawn, "==h==", "h", "==");
draws!(ruby_is_drawn, "{b}(r)", "b", "{");
draws!(superscript_is_drawn, "20^th^", "th", "^");

// What the renderer cannot draw keeps its text.

draws!(
    a_definition_list_keeps_its_text,
    "term\n: def",
    "def",
    "\u{0}"
);
draws!(raw_html_keeps_its_text, "<b>x</b>", "x", "\u{0}");
draws!(
    a_footnote_keeps_its_text,
    "text[^1]\n\n[^1]: note",
    "text",
    "\u{0}"
);
#[test]
fn latex_is_read_as_math() -> Result<(), String> {
    #[component]
    fn Probe() -> impl IntoElement {
        typr::ui::tokens::install();
        typr::ui::prose::prose().source("about \\(x\\)")
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/about/")?;
    assert!(
        !app.screen().texts().join(" ").contains("\\("),
        "the TeX delimiters are gone"
    );
    Ok(())
}

// The spec's two "leave it alone" scenarios.

draws!(
    equals_inside_code_is_left_alone,
    "`a == b`",
    "a == b",
    "==a =="
);
draws!(
    parentheses_alone_are_not_a_gloss,
    "mitosis (cell division)",
    "mitosis \\(cell division\\)",
    "{"
);

/// Every construct in the one list, with the test that draws it. A row without a case fails this, and a case
/// for a construct the list does not hold fails it too.
#[test]
fn every_supported_construct_has_a_case() {
    let cases: &[(&str, &str)] = &[
        ("strong", "strong_is_drawn"),
        ("emphasis", "emphasis_is_drawn"),
        ("strikethrough", "strikethrough_is_drawn"),
        ("inline-code", "inline_code_is_drawn"),
        ("code-fence", "a_code_fence_is_drawn"),
        ("table", "a_table_is_drawn"),
        ("task-list", "a_task_list_is_drawn"),
        ("link", "a_link_is_drawn"),
        ("image", "an_image_is_drawn"),
        ("heading", "a_heading_is_drawn"),
        ("list", "a_list_is_drawn"),
        ("quote", "a_quote_is_drawn"),
        ("highlight", "highlight_is_drawn"),
        ("ruby", "ruby_is_drawn"),
        ("superscript", "superscript_is_drawn"),
        ("subscript", "subscript_is_drawn"),
        ("definition-list", "a_definition_list_keeps_its_text"),
        ("inline-math", "inline_math_is_drawn"),
        ("display-math", "display_math_is_drawn"),
        ("raw-html", "raw_html_keeps_its_text"),
        ("footnote", "a_footnote_keeps_its_text"),
        ("latex", "latex_is_read_as_math"),
    ];
    for (construct, _) in SUPPORTED {
        assert!(
            cases.iter().any(|(name, _)| name == construct),
            "{construct} is in the list with no case"
        );
    }
    for (construct, _) in cases {
        assert!(
            SUPPORTED.iter().any(|(name, _)| name == construct),
            "{construct} has a case but is not in the list"
        );
    }
}
