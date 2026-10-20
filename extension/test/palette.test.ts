import { describe, expect, it } from "vitest";
import { accentCss, browserTheme, darkReaderTheme, luminance, paletteFromColorsJson, pageFix, ROLE_NAMES, seedPalette, wantsDark } from "../src/shared/palette";
import { defaults } from "../src/shared/settings";

describe("palette", () => {
  it("seed palettes have every role in both schemes", () => {
    const p = seedPalette("#4285F4", true);
    for (const r of ROLE_NAMES) {
      expect(p.dark[r], r).toMatch(/^#[0-9A-F]{6}$/);
      expect(p.light[r], r).toMatch(/^#[0-9A-F]{6}$/);
    }
    expect(luminance(p.dark.surface)).toBeLessThan(0.1);
    expect(luminance(p.light.surface)).toBeGreaterThan(0.8);
  });

  it("imports an end-4 colors.json and generates the other scheme", () => {
    const p = paletteFromColorsJson({ primary: "#ffb4ab", background: "#1a1111", surface: "#1a1111", on_surface: "#f1dedc" });
    expect(p?.shellDark).toBe(true);
    expect(p?.dark.surface).toBe("#1A1111");
    expect(p?.dark.on_surface).toBe("#F1DEDC");
    expect(luminance(p!.light.surface)).toBeGreaterThan(0.6);
    expect(paletteFromColorsJson({ nope: 1 })).toBeNull();
  });

  it("mode resolution", () => {
    const p = seedPalette("#6750A4", false);
    const s = defaults();
    expect(wantsDark(p, s)).toBe(false);
    expect(wantsDark({ ...p, shellDark: true }, s)).toBe(true);
    expect(wantsDark(p, { ...s, mode: "dark" })).toBe(true);
    expect(wantsDark(p, { ...s, mode: "dark" }, { mode: "off" })).toBeNull();
  });

  it("maps the palette onto Dark Reader and the browser theme", () => {
    const p = seedPalette("#6750A4", true);
    const s = { ...defaults(), brightness: 110 };
    const t = darkReaderTheme(p.dark, true, s, { contrast: 90, background: "#000000" });
    expect(t.mode).toBe(1);
    expect(t.darkSchemeBackgroundColor).toBe("#000000");
    expect(t.darkSchemeTextColor).toBe(p.dark.on_surface);
    expect(t.brightness).toBe(110);
    expect(t.contrast).toBe(90);
    expect(darkReaderTheme(p.light, false, s).lightSchemeBackgroundColor).toBe(p.light.surface);
    const bt = browserTheme(p.dark, true) as unknown as { colors: Record<string, string>; properties: Record<string, string> };
    expect(bt.colors.tab_line).toBe(p.dark.primary);
    expect(bt.colors.toolbar).toBe(p.dark.surface);
    expect(bt.properties.color_scheme).toBe("dark");
  });

  it("our CSS follows the options", () => {
    const p = seedPalette("#6750A4", true);
    const s = defaults();
    expect(accentCss(p.dark, true, s)).toContain(p.dark.primary);
    expect(accentCss(p.dark, true, { ...s, accents: false, scrollbars: false, excludeVideo: false })).toBe("");
    expect(accentCss(p.dark, true, { ...s, dimImages: 20 })).toContain("brightness(0.8)");
    expect(accentCss(p.dark, false, { ...s, dimImages: 20 })).not.toContain("brightness");
    const fix = pageFix({ invert: ["a"], css: "b{}", ignoreInlineStyle: [], ignoreImageAnalysis: [] }, p.dark, true, s, { css: "c{}" });
    expect(fix.invert).toEqual(["a"]);
    expect(fix.css.startsWith("b{}")).toBe(true);
    expect(fix.css.endsWith("c{}")).toBe(true);
    expect(fix.ignoreImageAnalysis).toContain("video");
  });
});
