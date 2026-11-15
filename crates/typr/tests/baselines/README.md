# Scene baselines

One picture per scene: the app draws the scene and `crates/typr/tests/visual.rs` compares the result
with the file here. A channel may move by 2 and at most 0.05% of the pixels may differ (1229 of the
1920×1280 picture), so a rasteriser that rounds differently passes while a spacing step, a radius or a
colour that repainted a region fails. A change confined to fewer than about 1200 pixels passes: that
is the price of not failing on rounding, and it is why a diff worth committing still gets looked at.

## Recording

```
TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual
```

That redraws every scene and overwrites its baseline. Record after an intended visual change, and
look at the diff before you commit it — the test cannot tell a fix from a regression.

A missing baseline fails the test rather than passing quietly, naming the file it wanted.

## Which machine

These pictures were recorded on macOS (darwin 27.2.0, arm64) at commit `884a2f8`, `history.png` again
at `5cbece8`, with the fonts that machine has. Font rasterisation and hinting differ between platforms,
so a Linux CI image reports every scene differing until it records its own set:

```
TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual   # on the CI image
```

No CI image has recorded a set yet, so the comparison runs as the last step of the workflow, after the
snapshot gate, and its first Linux run is expected to fail with that command in its output. Commit the
set from the image, or leave the comparison out of the pipeline for that platform; what must not happen
is a baseline recorded somewhere else and a check that always fails.

The zone cannot drift: `visual.rs` draws every scene with `TZ=UTC`, because the history scene prints its
moments with the local offset. Two differences — a platform that changes how a glyph is hinted, and a
platform that has not recorded a set — are the only ones these pictures can show.

## Adding a scene

Register the scene in the scene table in `crates/typr/src/main.rs`, then record the baselines again.
The test reads the scene list from `--list-scenes`, so a scene with no baseline fails until it is
recorded.
