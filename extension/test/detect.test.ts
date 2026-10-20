// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { pageIsDark } from "../src/shared/detect";

describe("already-dark detection", () => {
  it("reads the body background and color-scheme meta", () => {
    document.body.style.backgroundColor = "rgb(18, 18, 18)";
    expect(pageIsDark(document)).toBe(true);
    document.body.style.backgroundColor = "rgb(250, 250, 250)";
    expect(pageIsDark(document)).toBe(false);
    document.head.innerHTML = '<meta name="color-scheme" content="dark">';
    expect(pageIsDark(document)).toBe(true);
  });
});
