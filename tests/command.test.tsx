// The command palette on a real host: `nsh test --target macos` drives these in an AppKit window,
// where the search field does take the keyboard focus. They choose Settings, which writes nothing,
// because the real host uses the library in the user's data folder.

import { platform } from "nsh";
import { expect, open, test } from "nsh/test";

const ROW = "Edit the selected card";

test("the palette opens on its chord and Escape closes it", async () => {
  // A headless run gives the search field no keyboard focus, so the keys cannot reach the list.
  if (!platform.desktop) return;
  const app = await open("..");
  await app.waitFor("text:All cards");
  await app.keyboard.press("meta+k");
  await app.waitFor(ROW);
  expect(app.texts()).toContain("Review cards that are due");
  await app.keyboard.press("escape");
  await app.waitFor("text:All cards");
  expect(app.texts()).toContain("All cards");
});

test("typing narrows the palette and Return runs the row it is on", async () => {
  if (!platform.desktop) return;
  const app = await open("..");
  await app.waitFor("text:All cards");
  await app.keyboard.press("meta+k");
  await app.waitFor(ROW);
  // Settings is bound to `,`, and choosing it writes nothing.
  await app.type("text:Run a command", "settings");
  await app.waitFor("Settings");
  await app.keyboard.press("enter");
  await app.waitFor("text:Review");
  expect(app.texts()).toContain("Apply");
});

test("the shortcut panel lists the bindings and runs nothing", async () => {
  if (!platform.desktop) return;
  const app = await open("..");
  await app.waitFor("text:All cards");
  await app.keyboard.press("shift+/");
  await app.waitFor("Keyboard shortcuts");
  expect(app.texts()).toContain("Review cards that are due");
  await app.keyboard.press("enter");
  expect(app.texts()).toContain("Keyboard shortcuts");
  // The same key closes it, as `keymap-panel` says: a panel opens and closes on one key.
  await app.keyboard.press("shift+/");
  await app.waitFor("text:Keyboard shortcuts", { state: "gone" });
});

test("a command runs from the highlighted row", async () => {
  // The library this opens holds two cards, so a new card's editor is a write into a throwaway
  // library: `nsh test` is run with TYPR_DATA_DIR pointing at it.
  if (!platform.desktop) return;
  const app = await open("..");
  await app.waitFor("text:All cards");
  await app.keyboard.press("meta+k");
  await app.waitFor(ROW);
  // The third row is "New card" on the library surface: `s`, `n`, then `c`.
  await app.keyboard.press("down");
  await app.keyboard.press("down");
  await app.keyboard.press("enter");
  await app.waitFor("text:Attach image…");
  expect(app.texts()).not.toContain(ROW);
});
