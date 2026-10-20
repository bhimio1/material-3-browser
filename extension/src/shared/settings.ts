import type { Settings, SiteOverride } from "./types";
import { bestMatch, matchesAny } from "./match";

export const SETTINGS_VERSION = 2;

export const DEFAULT_SPEED_DIAL = [
  { title: "GitHub", url: "https://github.com" },
  { title: "YouTube", url: "https://www.youtube.com" },
  { title: "Arch Wiki", url: "https://wiki.archlinux.org" },
  { title: "Reddit", url: "https://www.reddit.com" },
  { title: "Wikipedia", url: "https://www.wikipedia.org" },
  { title: "AUR", url: "https://aur.archlinux.org" },
];

export function defaults(): Settings {
  return {
    version: SETTINGS_VERSION,
    mode: "follow",
    listMode: "blacklist",
    blacklist: [],
    whitelist: [],
    skipDarkSites: true,
    brightness: 100,
    contrast: 100,
    sepia: 0,
    grayscale: 0,
    accents: true,
    dimImages: 0,
    excludeVideo: true,
    scrollbars: true,
    font: { enabled: false, family: "Inter" },
    sites: {},
    browserTheme: true,
    seed: "#6750A4",
    importedPalette: null,
    newtab: { search: "", showWallpaper: true, clock24: true, showRecent: true, speedDial: DEFAULT_SPEED_DIAL.map((d) => ({ ...d })) },
  };
}

const num = (v: unknown, d: number, min: number, max: number) =>
  typeof v === "number" && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : d;
const bool = (v: unknown, d: boolean) => (typeof v === "boolean" ? v : d);
const strList = (v: unknown) => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string" && x.trim() !== "").map((x) => x.trim()) : []);
const hex = (v: unknown) => (typeof v === "string" && /^#[0-9a-f]{6}$/i.test(v) ? v : undefined);

function site(o: unknown): SiteOverride | null {
  if (!o || typeof o !== "object") return null;
  const r = o as Record<string, unknown>;
  const out: SiteOverride = {};
  if (r.mode === "dark" || r.mode === "light" || r.mode === "off") out.mode = r.mode;
  for (const k of ["brightness", "contrast", "sepia", "grayscale"] as const) {
    if (typeof r[k] === "number") out[k] = num(r[k], 100, 0, k === "brightness" || k === "contrast" ? 150 : 100);
  }
  if (hex(r.background)) out.background = hex(r.background);
  if (hex(r.text)) out.text = hex(r.text);
  if (typeof r.css === "string" && r.css.trim()) out.css = r.css;
  return out;
}

/**
 * Bring stored (or imported) settings up to date: fill in defaults, drop
 * bad values, and convert older layouts.
 *   v1 called the blacklist `disabledFor` and stored one global `dark` flag.
 */
export function migrate(raw: unknown): Settings {
  const d = defaults();
  if (!raw || typeof raw !== "object") return d;
  const r = { ...(raw as Record<string, unknown>) };
  if ((r.version ?? 1) === 1) {
    if (Array.isArray(r.disabledFor) && !Array.isArray(r.blacklist)) r.blacklist = r.disabledFor;
    if (typeof r.dark === "boolean" && r.mode === undefined) r.mode = r.dark ? "dark" : "light";
  }
  const nt = (r.newtab && typeof r.newtab === "object" ? r.newtab : {}) as Record<string, unknown>;
  const sites: Record<string, SiteOverride> = {};
  if (r.sites && typeof r.sites === "object") {
    for (const [k, v] of Object.entries(r.sites as Record<string, unknown>)) {
      const s = site(v);
      if (k.trim() && s) sites[k.trim()] = s;
    }
  }
  const speedDial = Array.isArray(nt.speedDial)
    ? nt.speedDial
        .filter((x): x is { title: unknown; url: unknown } => !!x && typeof x === "object")
        .filter((x) => typeof x.url === "string" && /^https?:\/\//.test(x.url))
        .map((x) => ({ title: typeof x.title === "string" ? x.title : String(x.url), url: x.url as string }))
    : d.newtab.speedDial;
  const font = (r.font && typeof r.font === "object" ? r.font : {}) as Record<string, unknown>;
  return {
    version: SETTINGS_VERSION,
    mode: ["follow", "dark", "light", "off"].includes(r.mode as string) ? (r.mode as Settings["mode"]) : d.mode,
    listMode: r.listMode === "whitelist" ? "whitelist" : "blacklist",
    blacklist: strList(r.blacklist),
    whitelist: strList(r.whitelist),
    skipDarkSites: bool(r.skipDarkSites, d.skipDarkSites),
    brightness: num(r.brightness, 100, 50, 150),
    contrast: num(r.contrast, 100, 50, 150),
    sepia: num(r.sepia, 0, 0, 100),
    grayscale: num(r.grayscale, 0, 0, 100),
    accents: bool(r.accents, d.accents),
    dimImages: num(r.dimImages, 0, 0, 50),
    excludeVideo: bool(r.excludeVideo, d.excludeVideo),
    scrollbars: bool(r.scrollbars, d.scrollbars),
    font: { enabled: bool(font.enabled, false), family: typeof font.family === "string" && font.family.trim() ? font.family : "Inter" },
    sites,
    browserTheme: bool(r.browserTheme, d.browserTheme),
    seed: hex(r.seed) ?? d.seed,
    importedPalette: r.importedPalette && typeof r.importedPalette === "object" ? (r.importedPalette as Settings["importedPalette"]) : null,
    newtab: {
      search: typeof nt.search === "string" ? nt.search : "",
      showWallpaper: bool(nt.showWallpaper, true),
      clock24: bool(nt.clock24, true),
      showRecent: bool(nt.showRecent, true),
      speedDial,
    },
  };
}

/** Is theming allowed for this URL by the blacklist / whitelist? */
export function listAllows(s: Settings, url: string): boolean {
  return s.listMode === "whitelist" ? matchesAny(s.whitelist, url) : !matchesAny(s.blacklist, url);
}

/** The per-site override for a URL, if any (most specific pattern wins). */
export function siteOverride(s: Settings, url: string): { pattern: string; override: SiteOverride } | undefined {
  const pattern = bestMatch(Object.keys(s.sites), url);
  return pattern ? { pattern, override: s.sites[pattern] } : undefined;
}

/** Add or remove a site from whichever list is active. */
export function setSiteThemed(s: Settings, site: string, themed: boolean): Settings {
  const next = structuredClone(s);
  if (next.listMode === "whitelist") {
    next.whitelist = next.whitelist.filter((p) => p !== site);
    if (themed) next.whitelist.push(site);
  } else {
    next.blacklist = next.blacklist.filter((p) => p !== site);
    if (!themed) next.blacklist.push(site);
  }
  return next;
}
