//! The visual language, checked against a picture.
//!
//! Every scene in the app is drawn and compared with the baseline recorded beside this file. A
//! baseline that is missing fails: a screen nobody recorded is a screen nobody checked. Set
//! `TYPR_UPDATE_BASELINES=1` to record the current drawing as the baseline — see
//! `tests/baselines/README.md` for when that is the right thing to do.

use std::path::{Path, PathBuf};
use std::process::Command;

use image::RgbImage;

/// A channel may differ by this much and still read as the same colour: antialiasing and the
/// rasteriser's rounding move a pixel by a step or two without a reader noticing.
const CHANNEL_TOLERANCE: u8 = 2;

/// At most this share of the picture may differ. Rendering the same scene twice on the same
/// machine produces identical pixels, so this is headroom for a rasteriser that rounds differently,
/// not for a design change: the corner radius moving from 6 to 10 points already changes 0.17% of a
/// 1200×800 picture.
const DIFFERING_SHARE: f64 = 0.0005;

fn baselines() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/baselines")
}

fn scenes() -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_typr"))
        .arg("--list-scenes")
        .output()
        .expect("the app lists its scenes");
    assert!(
        output.status.success(),
        "--list-scenes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Draw one scene to a PNG.
fn snap(scene: &str, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_typr"))
        .args(["--scene", scene, "--snap"])
        .arg(out)
        .args(["--frames", "4"])
        .output()
        .expect("the app snaps its scene");
    assert!(
        output.status.success(),
        "snapping {scene} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.exists(), "snapping {scene} wrote no picture");
}

fn updating() -> bool {
    std::env::var("TYPR_UPDATE_BASELINES").is_ok_and(|value| !value.is_empty() && value != "0")
}

/// The first pixel that differs: where it is, and its colour on each side.
type Spot = ([u32; 2], [u8; 3], [u8; 3]);

/// How many pixels differ by more than the tolerance, and the first one that does, so a failure
/// says where to look.
fn difference(want: &RgbImage, got: &RgbImage) -> (usize, Option<Spot>) {
    let mut differing = 0usize;
    let mut first = None;
    for (x, y, pixel) in got.enumerate_pixels() {
        let other = want.get_pixel(x, y);
        let apart =
            (0..3).any(|channel| other.0[channel].abs_diff(pixel.0[channel]) > CHANNEL_TOLERANCE);
        if apart {
            differing += 1;
            if first.is_none() {
                first = Some(([x, y], other.0, pixel.0));
            }
        }
    }
    (differing, first)
}

#[test]
fn every_scene_matches_its_baseline() {
    let scenes = scenes();
    assert!(!scenes.is_empty(), "the app lists no scene");
    let update = updating();
    let dir = baselines();
    let drawn = tempfile::tempdir().expect("a temp dir");
    let mut failures = Vec::new();

    for scene in &scenes {
        let shot = drawn.path().join(format!("{scene}.png"));
        snap(scene, &shot);
        let baseline = dir.join(format!("{scene}.png"));
        if update {
            std::fs::create_dir_all(&dir).expect("the baseline folder");
            std::fs::copy(&shot, &baseline).expect("the baseline is written");
            continue;
        }
        if !baseline.exists() {
            failures.push(format!(
                "{scene}: no baseline at {}\n  record every baseline with TYPR_UPDATE_BASELINES=1 cargo test --test visual",
                baseline.display()
            ));
            continue;
        }
        let want = image::open(&baseline)
            .expect("the baseline reads")
            .to_rgb8();
        let got = image::open(&shot).expect("the scene reads").to_rgb8();
        if want.dimensions() != got.dimensions() {
            failures.push(format!(
                "{scene}: the scene is {:?}, the baseline {:?}",
                got.dimensions(),
                want.dimensions()
            ));
            continue;
        }
        let (differing, first) = difference(&want, &got);
        let total = u64::from(got.width()) * u64::from(got.height());
        let share = differing as f64 / total as f64;
        if share > DIFFERING_SHARE {
            let at = first.map_or_else(String::new, |(at, want, got)| {
                format!(
                    " first at ({}, {}): baseline {want:?}, scene {got:?}",
                    at[0], at[1]
                )
            });
            failures.push(format!(
                "{scene}: {differing} of {total} pixels differ ({:.2}%){at}",
                share * 100.0
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} scene(s) do not match:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
