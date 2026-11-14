# A module with a folder keeps its root inside it

## Why

Two shapes exist for the same idea. `crates/typr-core/src/store/` holds `mod.rs` beside `tests/`; but
`crates/typr-core/src/hidden.rs` sits next to `crates/typr-core/src/hidden/tests.rs`, so the directory exists
only to hold tests while the module's code lives beside it. Eleven modules are in that second shape, and one
of them (`crates/typr/src/screen/`) is an empty directory left over from an earlier move.

A reader who sees a folder named after a module has to check twice whether the code is in the folder or
beside it.

## What Changes

- Every module that has a folder keeps its root inside the folder: `foo.rs` + `foo/tests.rs` becomes
  `foo/mod.rs` + `foo/tests.rs`. Eleven modules: `attachments`, `editor`, `history`, `library`, `logging`,
  `study` and `library/tile` in the application crate; `hidden`, `schedule`, `session` and `store/task` in
  the backend crate.
- The leftover empty `crates/typr/src/screen/` directory is deleted.
- No module declaration, import, test or production line changes: `mod foo;` resolves to `foo/mod.rs` exactly
  as it resolved to `foo.rs`.

## Impact

- Affected specs: `project-structure` (the test-placement requirement now says both files live in the folder).
- Affected code: eleven file moves under `crates/`, one directory removed. No content edits.
