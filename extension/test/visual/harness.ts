// Test-only bundle: applies the content theming to a plain page (no
// WebExtension APIs) so fixture pages can be rendered in Chromium.
import * as DarkReader from "darkreader";
import { darkReaderTheme, pageFix, paletteFromColorsJson } from "../../src/shared/palette";
import { defaults } from "../../src/shared/settings";
import { pageIsDark } from "../../src/shared/detect";
import type { Settings } from "../../src/shared/types";

interface Options {
  dark?: boolean;
  settings?: Partial<Settings>;
}

(window as unknown as { m3Theme: unknown }).m3Theme = (colors: unknown, opts: Options = {}) => {
  const palette = paletteFromColorsJson(colors);
  if (!palette) return "bad palette";
  const s = { ...defaults(), ...opts.settings } as Settings;
  const dark = opts.dark ?? palette.shellDark;
  if (dark && s.skipDarkSites && pageIsDark(document)) return "skipped (already dark)";
  const roles = dark ? palette.dark : palette.light;
  DarkReader.setFetchMethod(window.fetch);
  DarkReader.enable(darkReaderTheme(roles, dark, s) as Partial<DarkReader.Theme>, pageFix(undefined, roles, dark, s) as DarkReader.DynamicThemeFix);
  return "themed";
};
