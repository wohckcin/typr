# Tasks — FSRS scheduling

## 1. The algorithm

- [x] 1.1 New `crates/typr-core/src/fsrs.rs`: `pub struct Memory { stability: f64, difficulty: f64 }`, `pub const DEFAULT_WEIGHTS: [f64; 19]`, `pub fn initial(grade: Grade, weights: &[f64]) -> Memory`, `pub fn update(memory: Memory, grade: Grade, elapsed_days: f64, weights: &[f64]) -> Memory`, `pub fn interval_days(stability: f64, retention: f64) -> f64`.
  - **Verification**: `cargo test -p typr-core --lib fsrs` runs a golden table of published reference values for both grades at several elapsed times, plus a stability monotonicity test and a retention test (95% yields a shorter interval than 70%).
- [x] 1.2 Parsing and validation: `pub fn weights(text: &str) -> Result<Vec<f64>, StoreError>` refusing a count other than nineteen, a non-number and a non-positive value.
  - **Verification**: `a_malformed_weight_list_is_refused_without_replacing_the_old_one`.

## 2. Wiring

- [ ] 2.1 `Scheduler` gains `Fsrs { weights: Vec<f64>, retention: f64 }` implementing the same trait, mapping forgot to `Again` and remembered to `Good`, and bootstrapping stability from `interval_ms` when it is absent.
  - **Verification**: `an_fsrs_grade_bootstraps_a_prompt_it_has_never_seen` and `the_fixed_scheduler_still_produces_the_same_numbers`.
- [ ] 2.2 Migration V11: `prompt.stability REAL`, `prompt.difficulty REAL`, and settings rows `scheduler = 'fixed'`, `retention = 90`, `fsrs_weights = <defaults>`.
  - **Verification**: `a_library_from_before_fsrs_keeps_its_schedules` rewinds a real file to `user_version = 10` and reopens; `the_scheduler_defaults_to_fixed`.
- [ ] 2.3 `Db::grade` reads the prompt's memory state, passes it through the chosen scheduler and writes it back with the outcome, in the same transaction as the record.
  - **Verification**: `fsrs_state_survives_reopening_the_library` and `switching_schedulers_leaves_every_record_alone`.

## 3. Settings and export

- [ ] 3.1 Commands `SetScheduler { kind }`, `SetRetention { percent }`, `SetWeights { list }`, each refusing an invalid value through `StoreError::Setting`.
  - **Verification**: `a_retention_of_one_hundred_and_fifty_percent_is_refused`, `an_unknown_scheduler_is_refused`.
- [ ] 3.2 `Db::export_reviews() -> Result<String, StoreError>` producing the optimiser's CSV.
  - **Verification**: `the_export_holds_every_record_with_its_grade` counts rows against `review_log` and checks the header.

## 4. Close out

- [ ] 4.1 README documents the two schedulers, the retention default and the export; the settings screen (a later change) is named as where they will be exposed.
  - **Verification**: the README section exists and names both settings keys.
- [ ] 4.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the changes before it.
- Next roadmap row: `mochi-import-export`.
