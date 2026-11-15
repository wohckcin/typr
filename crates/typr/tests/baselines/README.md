# Scene baselines

One picture per scene: the app draws the scene and `crates/typr/tests/visual.rs` compares the result
with the file here. A channel may move by 2 and at most 0.05% of the pixels may differ, so a rasteriser
that rounds differently passes while a token, a padding step or a colour that changed fails.

## Recording

```
TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual
```

That redraws every scene and overwrites its baseline. Record after an intended visual change, and
look at the diff before you commit it — the test cannot tell a fix from a regression.

A missing baseline fails the test rather than passing quietly, naming the file it wanted.

## Which machine

These pictures were recorded on macOS (darwin 27.2.0, arm64), with the fonts that machine has, at
commit `884a2f8`. No CI image has recorded a set yet, so the `Compare every scene with its baseline`
step will report differences on its first run — that is the step telling you to record one there. Font rasterisation
and hinting differ between platforms, so a Linux CI image will report every scene differing until it
records its own set:

```
TYPR_UPDATE_BASELINES=1 cargo test -p typr --test visual   # on the CI image
```

Commit that set from the image, or keep the visual test out of the pipeline for that platform; what
must not happen is a baseline recorded somewhere else and a check that always fails.

## Adding a scene

Register the scene in the scene table in `crates/typr/src/main.rs`, then record the baselines again.
The test reads the scene list from `--list-scenes`, so a scene with no baseline fails until it is
recorded.
