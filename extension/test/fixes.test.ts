import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { fixFor, parseFixes } from "../src/shared/fixes";

const SAMPLE = `*

INVERT
.generic-icon

CSS
::placeholder { opacity: 0.5 !important; }

================================

*.abc.net.au

INVERT
[data-component="ABCNewsLogo"] *

IGNORE INLINE STYLE
[data-component="ABCLogo"] *

================================

*.acesso.gov.pt

================================

example.com
example.org

CSS
.picture {
    background-color: rgba(255, 255, 255, 1) !important;
}

IGNORE IMAGE ANALYSIS
.hero
`;

describe("Dark Reader site fixes", () => {
  it("parses blocks and skips URL-only ones", () => {
    const entries = parseFixes(SAMPLE);
    expect(entries.map((e) => e.url)).toEqual([["*"], ["*.abc.net.au"], ["example.com", "example.org"]]);
    expect(entries[2].css).toContain("background-color");
    expect(entries[2].ignoreImageAnalysis).toEqual([".hero"]);
  });

  it("merges the generic fix with the site's", () => {
    const entries = parseFixes(SAMPLE);
    const fix = fixFor(entries, "https://www.abc.net.au/news")!;
    expect(fix.invert).toEqual([".generic-icon", '[data-component="ABCNewsLogo"] *']);
    expect(fix.ignoreInlineStyle).toEqual(['[data-component="ABCLogo"] *']);
    expect(fixFor(entries, "https://unknown.site/")!.invert).toEqual([".generic-icon"]);
  });

  it("parses the bundled config", () => {
    const entries = parseFixes(readFileSync("data/dynamic-theme-fixes.config", "utf8"));
    expect(entries.length).toBeGreaterThan(1000);
    expect(entries[0].url).toContain("*");
    const github = fixFor(entries, "https://github.com/");
    expect(github).toBeDefined();
  });
});
