/** Material 3 roles as end-4 / matugen name them (snake_case → #RRGGBB). */
export type Roles = Record<string, string>;

/** What the native host sends (and what the extension falls back to). */
export interface PaletteMessage {
  type: "palette";
  source: "end4" | "seed" | "imported";
  /** Which scheme the desktop is using right now. */
  shellDark: boolean;
  dark: Roles;
  light: Roles;
}

export type Mode = "follow" | "dark" | "light" | "off";

export interface SiteOverride {
  mode?: "dark" | "light" | "off";
  brightness?: number;
  contrast?: number;
  sepia?: number;
  grayscale?: number;
  /** Custom background / text colors (#RRGGBB) instead of the palette's. */
  background?: string;
  text?: string;
  css?: string;
}

export interface SpeedDialItem {
  title: string;
  url: string;
}

export interface Settings {
  version: number;
  mode: Mode;
  listMode: "blacklist" | "whitelist";
  /** Sites never themed (blacklist mode). */
  blacklist: string[];
  /** The only sites themed (whitelist mode). */
  whitelist: string[];
  /** Leave sites that are already dark alone (in dark mode). */
  skipDarkSites: boolean;
  brightness: number;
  contrast: number;
  sepia: number;
  grayscale: number;
  /** Primary color for links, focus rings, checkboxes and carets. */
  accents: boolean;
  /** Dim images in dark mode, 0–50 %. */
  dimImages: number;
  /** Don't touch video players. */
  excludeVideo: boolean;
  scrollbars: boolean;
  font: { enabled: boolean; family: string };
  /** Per-site overrides keyed by pattern. */
  sites: Record<string, SiteOverride>;
  /** Paint the browser UI with the palette. */
  browserTheme: boolean;
  /** Fallback seed when the palette host isn't reachable. */
  seed: string;
  /** A palette imported from a colors.json file (Flatpak without host). */
  importedPalette: PaletteMessage | null;
  newtab: {
    /** Search engine name ("" = browser default). */
    search: string;
    showWallpaper: boolean;
    clock24: boolean;
    showRecent: boolean;
    speedDial: SpeedDialItem[];
  };
}

/** Everything a content script needs to theme one page. */
export interface ContentState {
  enabled: boolean;
  /** Why theming is off ("blacklist", "paused", "off", "already-dark"…). */
  reason?: string;
  dark: boolean;
  theme?: Record<string, unknown>;
  fixes?: Fix;
  /** Check whether the site is already dark before theming. */
  detectDark: boolean;
}

export interface Fix {
  invert: string[];
  css: string;
  ignoreInlineStyle: string[];
  ignoreImageAnalysis: string[];
}
