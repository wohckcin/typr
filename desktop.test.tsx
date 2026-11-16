// The app on a real host: `nsh test --target macos` drives these in an AppKit window, and a plain
// `nsh test` runs them headless. They only read the app, because the real host uses the library in
// the user's data folder and a test must not write to it.

import { expect, open, test } from "nsh/test";

test("the app opens with its navigation", async () => {
  const app = await open(".");
  for (const label of ["Search", "All cards", "Utilities", "Settings", "Diagnostics"]) {
    expect(app.texts()).toContain(label);
  }
  // The library's state is said either way: a headless run has no data folder, a real one has.
  const stated = app.texts().filter((text) => text === "Library open" || text === "Library closed");
  expect(stated.length).toBe(1);
});

test("the search field is on the library screen and the chord reaches it", async () => {
  const app = await open(".");
  // `text:` matches an input's placeholder, which is the field's only name.
  await app.waitFor("text:Search cards");
  await app.keyboard.press("/");
  await app.type("text:Search cards", "ga");
  // The host's native field does not expose its text to selectors, so the text itself cannot be
  // read back there; the chord and the keystrokes are exercised and the screen must survive them.
  expect(app.texts()).toContain("All cards");
});

test("Settings opens and Escape returns to the library", async () => {
  const app = await open(".");
  await app.press("Settings");
  await app.waitFor("text:Review");
  expect(app.texts()).toContain("Apply");
  await app.keyboard.press("escape");
  await app.waitFor("text:All cards");
  expect(app.texts()).toContain("All cards");
});
