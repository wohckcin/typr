# Code-review record

The failing repros behind the 17 findings reported for `c77c60b` are gone. Every finding now has a
permanent test in the suite, named in `FINDINGS.md`, and the throwaway files stopped compiling the
moment the fixes changed a signature.

The reproducers are still in git if a reader wants them.

```sh
git show 643f652:interrogate/core-audit.rs
git show 643f652:interrogate/app-probes.rs
```

`FINDINGS.md` is the record that survives. It names each break, where it lived, and the test that
keeps it from coming back.

## What the repros proved at the time

Against `main` at `fdd8f97`, every repro but one passed: `core-adv_repro` 5, `core-adversarial` 4,
`core-audit` 4, `core-repro` 3, `core-verify_probe` 2 of 3. The odd one out,
`a_panic_in_a_batch_is_attributed_to_the_wrong_request`, failed because it asserted the broken
behaviour it was written to expose ("the second request got no answer at all"). The worker now
answers `answered=[1, 2, 0]`, giving the second request its own id.

## Findings with no permanent test

Grades were never timed, so no slow-command record could fire for the command the user waits on.
That is now covered by `a_grade_is_timed_like_any_other_command`.

The items the fourth adversary could not verify stay unclaimed, not findings: dependency log
filtering, the row-builder index panic, a grade in flight during a worker panic, and whether the
preview reflects side boundaries.
