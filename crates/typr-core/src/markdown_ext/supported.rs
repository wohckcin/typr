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

/// The constructs, in the order Mochi's shortcut panel and this app's cards use them, each as the NSH
/// renderer and the app's extensions draw it now (`design.md` of the markdown-coverage change records the
/// probe).
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
    ("highlight", Support::Rendered),
    ("ruby", Support::Rendered),
    // Smaller and lifted by a few points: the renderer has no baseline shift, so it approximates one.
    ("superscript", Support::Degraded),
    // The renderer reads `~…~` itself.
    ("subscript", Support::Rendered),
    // markdown-rs has no definition-list construct, so the text stays as written.
    ("definition-list", Support::Unsupported),
    // The renderer draws both as formulas.
    ("inline-math", Support::Rendered),
    ("display-math", Support::Rendered),
    ("raw-html", Support::Unsupported),
    ("footnote", Support::Unsupported),
    ("latex", Support::Unsupported),
];
