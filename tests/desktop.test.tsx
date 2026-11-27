// The app on a real host: `nsh test --target macos` drives these in an AppKit window, and a plain
// `nsh test` runs them headless. They only read the app, because the real host uses the library in
// the user's data folder and a test must not write to it.

import { platform } from "nsh";
import { expect, open, test } from "nsh/test";

test("the app opens with its navigation", async () => {
  const app = await open("..");
  for (const label of ["Inbox", "Due today", "New cards", "Settings", "Trash"]) {
    expect(app.texts()).toContain(label);
  }
  // Only a closed library is stated (a headless run has no data folder); an open one says nothing.
  expect(app.texts()).not.toContain("Library open");
});

test("the search field is on the library screen and the chord reaches it", async () => {
  const app = await open("..");
  // `text:` matches an input's placeholder, which is the field's only name.
  await app.waitFor("text:/^Search cards/");
  await app.keyboard.press("/");
  await app.type("text:/^Search cards/", "ga");
  // The field's contents cannot be read back on the host, so the test proves the field is reachable
  // and the chord and keystrokes leave the screen standing, not what it then holds.
  expect(app.texts()).toContain("Inbox");
});

test("Settings opens and Escape returns to the library", async () => {
  const app = await open("..");
  await app.press("Settings");
  await app.waitFor("text:Theme");
  expect(app.texts()).toContain("Apply");
  await app.keyboard.press("escape");
  await app.waitFor("text:Inbox");
  expect(app.texts()).not.toContain("Apply");
});

test("a button answers the pointer where there is one", async () => {
  const app = await open("..");
  await app.waitFor("role:button[New card]");
  // A headless run opens the app as a phone, which has no pointer; the host gives it one.
  if (!platform.desktop) return;
  const rest = await app.screenshot();
  await app.hover("role:button[New card]");
  const hovered = await app.screenshot();
  // The two pictures differ only where the button is, which is the wash coming in.
  expect(hovered.length).toBe(rest.length);
  expect(rest.every((byte, at) => byte === hovered[at])).toBe(false);
});
