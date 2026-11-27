#!/bin/sh
# Seed a throwaway library, open typr in the real macOS host, screenshot library/editor/settings in
# light and dark. Usage: .claude/skills/run-typr/shoot.sh <outdir>   (run from the repo root)
set -e
mkdir -p "${1:?outdir}"
out=$(cd "$1" && pwd)
lib="$out/lib"
# The seed is a temporary integration test: typr-core's Db is the only way to write a library.
cat > crates/typr/tests/zz_seed.rs <<RS
#[test]
fn seed() {
    let root = std::path::Path::new("$lib");
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db.apply(&typr_core::store::Command::CreateDeck { parent: None, name: "Spanish".into() }, 0) {
        Ok(typr_core::store::Outcome::Deck(id)) => id, o => panic!("{o:?}") };
    for m in ["gato\n---\ncat", "perro\n---\ndog", "casa\n---\nhouse"] {
        db.apply(&typr_core::store::Command::CreateCard { deck, markdown: m.into() }, 0).unwrap();
    }
}
RS
trap 'rm -f crates/typr/tests/zz_seed.rs' EXIT
cargo test -p typr --test zz_seed >/dev/null
# `nsh test nsh.json` with a script on stdin: the steps run against the app; --target macos opens a
# real window, which is what lets app.screenshot(path) draw (headless refuses).
cat > "$out/steps.js" <<JS
for (const scheme of ["light", "dark"]) {
  await app.setColorScheme(scheme);
  await app.waitFor("role:button[gato]");
  await app.screenshot("$out/library-" + scheme + ".png");
  await app.press("role:button[gato]");
  await app.waitFor("label:Card source");
  await app.screenshot("$out/editor-" + scheme + ".png");
  await app.keyboard.press("escape");
  await app.press("Settings");
  await app.waitFor("New cards a day");
  await app.screenshot("$out/settings-" + scheme + ".png");
  await app.keyboard.press("escape");
}
JS
TYPR_DATA_DIR="$lib" nsh test nsh.json --target macos < "$out/steps.js"
ls "$out"/*.png
