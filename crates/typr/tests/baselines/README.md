# Scene baselines

One picture per scene, in the folder named for the platform that drew it (`macos/`, `linux/`): the app
draws the scene and `crates/typr/tests/visual.rs` compares the result with the file for this platform. A channel may move by 2 and at most 0.05% of the pixels may differ (1229 of the
1920×1280 picture), so a rasteriser that rounds differently passes while a spacing step, a radius or a
colour that repainted a region fails. A change confined to fewer than about 1200 pixels passes: that
is the price of not failing on rounding, and it is why a diff worth committing still gets looked at.

## Recording

```
TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual
```

That redraws every scene and overwrites this platform's baselines. Record after an intended visual change, and
look at the diff before you commit it — the test cannot tell a fix from a regression.

A missing baseline fails the test rather than passing quietly, naming the file it wanted.

## Which machine

Fonts are rasterised by the platform, so each platform keeps its own set. `visual.rs` reads
`TYPR_BASELINE_SET` and falls back to `std::env::consts::OS`; CI names its set so a Linux machine
running the record command cannot overwrite the pictures the runner checks.

- `macos/` — this Mac (macOS 27.2.0, arm64), recorded at `884a2f8`, `history.png` again at `5cbece8`.
  Written by the local record command, which is why the folder is the platform name.
- `ubuntu-26.04/` — GitHub Actions image `ubuntu-26.04` build `20261111.196.1`, recorded at
  `fb3905e` by the `Record this runner's baselines` step (dispatch run `43672594363`) and committed from
  its artifact. `image.txt` holds the build that drew it (`ImageVersion`, `20261111.196.1`); the
  Record step writes it, and a step before the comparison fails when the runner reports a different
  build.

`ci.yml` pins `runs-on: ubuntu-26.04` because `ubuntu-latest` is an alias GitHub moves between
images — the run's annotation says it starts moving to Ubuntu 26 on 2026-10-19 — and a moved image
rasterises its fonts differently. Pinning the label pins the image's name, not its build: GitHub
rebuilds the image under the same label, and on a new build the comparison step fails with a message
saying the image changed rather than reporting 33 unexplained scenes. When that happens, re-record
from a dispatch run, download the artifact and commit it; the pictures the runner drew are the ones it
should check.

The zone cannot drift: `visual.rs` draws every scene with `TZ=UTC`, because the history scene prints its
moments with the local offset.

## Adding a scene

Register the scene in the scene table in `crates/typr/src/main.rs`, then record it in both sets:

- Locally: `TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual` writes this platform's set and
  leaves the others alone. `TYPR_BASELINE_SET=<name>` picks a set by hand.
- For CI: dispatch the workflow with `record_baselines` on
  (`gh workflow run ci.yml -f record_baselines=true`), download `baselines-ubuntu-26.04`
  (`gh run download <run-id> -n baselines-ubuntu-26.04 -D /tmp/bl`), copy it over
  `tests/baselines/ubuntu-26.04/` and commit. Check the diff: only the new scene's file and
  `image.txt` should change. Anything else means the runner's rendering drifted, not that a scene was
  added.

The test reads the scene list from `--list-scenes`, so a scene with no baseline fails until it is
recorded.

## Adding a scene

Register the scene in the scene table in `crates/typr/src/main.rs`, then record the baselines again.
The test reads the scene list from `--list-scenes`, so a scene with no baseline fails until it is
recorded.
