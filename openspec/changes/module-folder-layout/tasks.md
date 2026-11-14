# Tasks — module folders

## 1. Move every folder-rooted module

- [x] 1.1 `git mv` the eleven module roots into their folders (`attachments`, `editor`, `history`, `library`,
      `library/tile`, `logging`, `study`, `hidden`, `schedule`, `session`, `store/task`) and delete the empty
      `crates/typr/src/screen/` directory.
  - **Verification**: `find crates -name '*.rs' -not -name 'mod.rs' | xargs -n1 dirname | sort | uniq -d`
    reports no file sitting beside a folder of its own name, and `cargo test --workspace --locked` passes.

## 2. Prove nothing changed but the paths

- [x] 2.1 Run the whole gate and confirm the test count is unchanged.
  - **Verification**: `cargo test --workspace --locked` reports the same 244 passing tests, `cargo fmt --check`
    is clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports nothing, and
    `git show --stat HEAD` lists only renames.

## Workflow follow-up

- [x] Sync the modified requirement into `openspec/specs/project-structure/spec.md` and archive this change.
