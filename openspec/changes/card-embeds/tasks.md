# Tasks: card embeds

## 1. Recognition

- [x] 1.1 Add `pub fn embeds(text: &str) -> Vec<String>` and `pub fn expand(text: &str, resolve: impl Fn(&str) -> Option<String>) -> String`
      to `crates/typr-core/src/markdown_ext.rs`, rewriting `![[Name]]` into a `typr-embed`
      fenced block outside fences and code spans.
      *Verification*: `![[A]]` yields one name; `` `![[A]]` `` and a fenced copy yield
      none; the body is not scanned for sides.

## 2. Rendering

- [x] 2.1 Draw a `typr-embed` block as an inset block with the source card's body and a
      caption naming it, in the editor preview and in a review face.
      *Verification*: a scene card embedding another draws the inset block and the caption,
      and its own text still renders around it.

- [ ] 2.2 Draw a missing target as the name in the danger tone with the caption "no such
      card", leaving the rest of the card intact.
      *Verification*: an embed naming an absent card draws the marker and the card's other
      text.

- [ ] 2.3 Stop at one level: an embed inside an embedded card draws `Name (embedded card)`.
      *Verification*: a self-embedding card and an A-embeds-B-embeds-A pair both draw one
      marker and no repeated body.

## 3. Verification

- [ ] 3.1 Prove no schedule moves: snapshot the prompts before and after adding an embed.
      *Verification*: the prompt rows and due dates are identical, and the embedding card's
      side count is unchanged.

- [ ] 3.2 Run `cargo test --workspace --locked`, `cargo fmt --check` and
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
      *Verification*: all three exit zero.

## Workflow follow-up

- [ ] 4.1 Archive with `openspec archive card-embeds --yes` once the tasks are checked.
