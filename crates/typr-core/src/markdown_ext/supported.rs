//! Every construct the app answers for, and how.
//!
//! One list, so a construct cannot be rendered by accident and left out of the answer: the renderer
//! tests name a case for each entry, and the tests fail when either side gains a row the other lacks.

/// What the app does with a construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// The toolkit's markdown renderer draws it as written.
    Rendered,
    /// The text is kept and the shape is approximated, because the renderer has no hook for it.
    Degraded,
    /// Anything else: the text stays on the page exactly as written.
    Unsupported,
}

/// The constructs, in the order Mochi's shortcut panel and this app's cards use them.
pub const SUPPORTED: &[(&str, Support)] = &[
    ("strong", Support::Rendered),
    ("emphasis", Support::Rendered),
    ("strikethrough", Support::Rendered),
    ("inline-code", Support::Rendered),
    ("code-fence", Support::Rendered),
    ("table", Support::Rendered),
    ("task-list", Support::Rendered),
    ("link", Support::Rendered),
    ("image", Support::Rendered),
    ("heading", Support::Rendered),
    ("list", Support::Rendered),
    ("quote", Support::Rendered),
    ("highlight", Support::Degraded),
    ("ruby", Support::Degraded),
    ("superscript", Support::Degraded),
    ("subscript", Support::Degraded),
    ("definition-list", Support::Degraded),
    ("inline-math", Support::Degraded),
    ("display-math", Support::Degraded),
    ("raw-html", Support::Unsupported),
    ("footnote", Support::Unsupported),
    ("latex", Support::Unsupported),
];
