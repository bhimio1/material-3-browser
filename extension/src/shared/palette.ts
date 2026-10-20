import { argbFromHex, hexFromArgb, Hct, MaterialDynamicColors, SchemeTonalSpot } from "@material/material-color-utilities";
import type { Fix, PaletteMessage, Roles, Settings, SiteOverride } from "./types";

/** The roles end-4's colors.json has, snake_case like matugen writes them. */
export const ROLE_NAMES = [
  "primary", "on_primary", "primary_container", "on_primary_container",
  "secondary", "on_secondary", "secondary_container", "on_secondary_container",
  "tertiary", "on_tertiary", "tertiary_container", "on_tertiary_container",
  "error", "on_error", "error_container", "on_error_container",
  "background", "on_background", "surface", "on_surface", "surface_variant", "on_surface_variant",
  "outline", "outline_variant", "shadow", "scrim",
  "inverse_surface", "inverse_on_surface", "inverse_primary",
  "surface_dim", "surface_bright",
  "surface_container_lowest", "surface_container_low", "surface_container",
  "surface_container_high", "surface_container_highest",
] as const;

const camel = (s: string) => s.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());

/** Generate both schemes from a seed color (tonal spot, like matugen's default). */
export function seedRoles(seed: string, dark: boolean): Roles {
  const hex = /^#[0-9a-f]{6}$/i.test(seed) ? seed : "#6750A4";
  const scheme = new SchemeTonalSpot(Hct.fromInt(argbFromHex(hex)), dark, 0);
  const colors = MaterialDynamicColors as unknown as Record<string, { getArgb(s: SchemeTonalSpot): number }>;
  const roles: Roles = {};
  for (const name of ROLE_NAMES) {
    const c = colors[camel(name)];
    if (c) roles[name] = hexFromArgb(c.getArgb(scheme)).toUpperCase();
  }
  return roles;
}

export function seedPalette(seed: string, systemDark: boolean): PaletteMessage {
  return { type: "palette", source: "seed", shellDark: systemDark, dark: seedRoles(seed, true), light: seedRoles(seed, false) };
}

/**
 * Turn an end-4 / matugen colors.json (one scheme) into a full palette,
 * generating the other scheme from its primary color.
 */
export function paletteFromColorsJson(json: unknown): PaletteMessage | null {
  if (!json || typeof json !== "object") return null;
  const map = json as Record<string, unknown>;
  const roles: Roles = {};
  for (const [k, v] of Object.entries(map)) {
    if (typeof v === "string" && /^#[0-9a-f]{6}$/i.test(v)) roles[k] = v.toUpperCase();
  }
  if (!roles.primary || !(roles.background || roles.surface)) return null;
  const dark = luminance(roles.background ?? roles.surface) < 0.2;
  const generated = seedRoles(roles.primary, dark);
  const current = { ...generated, ...roles };
  const other = seedRoles(roles.primary, !dark);
  return { type: "palette", source: "imported", shellDark: dark, dark: dark ? current : other, light: dark ? other : current };
}

export function rgb(hex: string): [number, number, number] {
  const v = parseInt(hex.replace("#", ""), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

export function rgba(hex: string, alpha: number): string {
  const [r, g, b] = rgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function luminance(hex: string): number {
  const f = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const [r, g, b] = rgb(hex);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

/** Which scheme a page gets: the site override, the global mode, or the desktop's. */
export function wantsDark(palette: PaletteMessage, settings: Settings, site?: SiteOverride): boolean | null {
  const mode = site?.mode ?? settings.mode;
  if (mode === "off") return null;
  if (mode === "dark") return true;
  if (mode === "light") return false;
  return palette.shellDark;
}

/** Dark Reader theme options from the palette. */
export function darkReaderTheme(roles: Roles, dark: boolean, settings: Settings, site?: SiteOverride): Record<string, unknown> {
  const bg = site?.background ?? roles.surface;
  const text = site?.text ?? roles.on_surface;
  return {
    mode: dark ? 1 : 0,
    brightness: site?.brightness ?? settings.brightness,
    contrast: site?.contrast ?? settings.contrast,
    sepia: site?.sepia ?? settings.sepia,
    grayscale: site?.grayscale ?? settings.grayscale,
    useFont: settings.font.enabled,
    fontFamily: settings.font.family,
    textStroke: 0,
    darkSchemeBackgroundColor: dark ? bg : roles.surface,
    darkSchemeTextColor: dark ? text : roles.on_surface,
    lightSchemeBackgroundColor: dark ? roles.surface : bg,
    lightSchemeTextColor: dark ? roles.on_surface : text,
    scrollbarColor: settings.scrollbars ? roles.outline : "",
    selectionColor: roles.primary_container,
    styleSystemControls: true,
  };
}

/** Our CSS on top of Dark Reader: Material 3 accents, focus, images, video. */
export function accentCss(roles: Roles, dark: boolean, settings: Settings): string {
  const css: string[] = [];
  if (settings.accents) {
    css.push(`
:root { accent-color: ${roles.primary} !important; }
input, textarea, [contenteditable] { caret-color: ${roles.primary} !important; }
:focus-visible { outline-color: ${roles.primary} !important; }
:is(p, li, td, dd, blockquote, figcaption, article, main) a:not([role="button"]):not(:has(img)) {
  color: ${roles.primary} !important;
  text-decoration-color: ${rgba(roles.primary, 0.5)} !important;
}`);
  }
  if (settings.scrollbars) {
    css.push(`html { scrollbar-color: ${roles.outline} ${roles.surface_container ?? roles.surface} !important; }`);
  }
  if (dark && settings.dimImages > 0) {
    css.push(`img:not([src$=".svg"]), picture, [role="img"] { filter: brightness(${(100 - settings.dimImages) / 100}) !important; }`);
  }
  if (settings.excludeVideo) {
    css.push(`video { filter: none !important; }`);
  }
  return css.join("\n");
}

/** Merge Dark Reader's site fix with our options and the per-site CSS. */
export function pageFix(base: Fix | undefined, roles: Roles, dark: boolean, settings: Settings, site?: SiteOverride): Fix {
  const fix: Fix = base
    ? { invert: [...base.invert], css: base.css, ignoreInlineStyle: [...base.ignoreInlineStyle], ignoreImageAnalysis: [...base.ignoreImageAnalysis] }
    : { invert: [], css: "", ignoreInlineStyle: [], ignoreImageAnalysis: [] };
  if (settings.excludeVideo) fix.ignoreImageAnalysis.push("video", "video *");
  fix.css = [fix.css, accentCss(roles, dark, settings), site?.css ?? ""].filter(Boolean).join("\n");
  return fix;
}

/** The browser UI theme (WebExtension theme API) from the palette. */
export function browserTheme(roles: Roles, dark: boolean): browser._manifest.ThemeType {
  return {
    colors: {
      frame: roles.surface_container,
      frame_inactive: roles.surface_container_low,
      tab_background_text: roles.on_surface,
      tab_selected: roles.secondary_container,
      tab_text: roles.on_secondary_container,
      tab_line: roles.primary,
      tab_loading: roles.primary,
      toolbar: roles.surface,
      toolbar_text: roles.on_surface,
      toolbar_field: roles.surface_container_highest,
      toolbar_field_text: roles.on_surface,
      toolbar_field_border: "transparent",
      toolbar_field_focus: roles.surface_container_highest,
      toolbar_field_border_focus: roles.primary,
      toolbar_field_highlight: roles.primary_container,
      toolbar_field_highlight_text: roles.on_primary_container,
      toolbar_top_separator: "transparent",
      toolbar_bottom_separator: "transparent",
      toolbar_vertical_separator: roles.outline_variant,
      popup: roles.surface_container_high,
      popup_text: roles.on_surface,
      popup_border: roles.outline_variant,
      popup_highlight: roles.secondary_container,
      popup_highlight_text: roles.on_secondary_container,
      sidebar: roles.surface_container_low,
      sidebar_text: roles.on_surface,
      sidebar_border: roles.outline_variant,
      sidebar_highlight: roles.secondary_container,
      sidebar_highlight_text: roles.on_secondary_container,
      button_background_hover: rgba(roles.on_surface, 0.08),
      button_background_active: rgba(roles.on_surface, 0.12),
      icons: roles.on_surface_variant,
      icons_attention: roles.primary,
      ntp_background: roles.surface,
      ntp_text: roles.on_surface,
    },
    properties: {
      color_scheme: dark ? "dark" : "light",
      content_color_scheme: dark ? "dark" : "light",
    },
  } as unknown as browser._manifest.ThemeType;
}

/** CSS custom properties for the extension's own pages. */
export function cssVars(roles: Roles): string {
  return Object.entries(roles)
    .map(([k, v]) => `--m3-${k.replace(/_/g, "-")}: ${v};`)
    .join(" ");
}
