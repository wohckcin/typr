// The library as a `nsh test` run sees it: no data folder, so the sidebar says the library is
// closed and the deck under it is empty. A Rust app's tests in TypeScript open the app's root with
// `open("..")`, and the steps are the same as a TSX app's.

import { expect, open, test } from "nsh/test";

test("a library that cannot open says so and still shows the app", async () => {
  const app = await open("..");
  await app.waitFor("text:/^Library closed/");
  // The rows are the app's navigation whatever the library's state.
  for (const label of ["Search", "Navigation", "All cards", "Utilities", "Settings", "Diagnostics"]) {
    expect(app.texts()).toContain(label);
  }
});

test("an empty deck offers the one action and says what is missing", async () => {
  const app = await open("..");
  await app.waitFor("text:This deck has no cards yet");
  expect(app.texts()).toContain("New card");
  // The action is a control, not a badge: the badge above the sentence is the wrong element for it.
  expect(app.count("role:button[New card]")).toBe(1);
});
