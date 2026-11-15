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

Fonts are rasterised by the platform, so each platform keeps its own set and the test reads
`std::env::consts::OS` to pick one. Neither set is the other's: a Mac drawing says nothing about a Linux
runner.

- `macos/` — macOS 27.2.0, arm64, recorded at `884a2f8`, `history.png` again at `5cbece8`.
- `linux/` — GitHub Actions image `ubuntu-26.04`, build `20261111.196.1`, recorded at `fb3905e` by the
  `Record this runner's baselines` step (`workflow_dispatch` with `record_baselines` on, run
  `43672594363`) and committed from its `baselines-linux` artifact. `ci.yml` pins `runs-on: ubuntu-26.04`
  for this reason: `ubuntu-latest` moved on 2026-10-19 and would repaint every picture.

The zone cannot drift: `visual.rs` draws every scene with `TZ=UTC`, because the history scene prints its
moments with the local offset. A platform with no set fails on its first run with the command that
records one; that is the step telling you to record it there.

## Adding a scene

Register the scene in the scene table in `crates/typr/src/main.rs`, then record the baselines again.
The test reads the scene list from `--list-scenes`, so a scene with no baseline fails until it is
recorded.
