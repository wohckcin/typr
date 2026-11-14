# Tasks — restore an archived deck

## 1. Prove the behaviour the spec now states

- [x] 1.1 Grade the card in the deck round-trip test and assert its stored due date is unchanged after the deck is restored.
  - **Verification**: `cargo test -p typr --bin typr -- a_card_under_an_archived_deck_is_reachable_and_the_deck_can_come_back` passes and the assertion reads the due date back through `History` both before archiving and after restoring.
- [x] 1.2 Archive this change so the requirement lands in `openspec/specs/card-library/spec.md`.
  - **Verification**: `openspec archive restore-archived-deck --yes` reports the created spec files, and `openspec validate card-library --type spec --strict` is valid.

## Workflow follow-up

- Check `interrogate/FINDINGS.md`: its "no requirement changed" note now has one exception, and the
  deck guard case it lists should stay listed.
