/**
 * Background: owns the palette (from the m3-browser native host, an
 * imported colors.json, or the seed), the settings, Dark Reader's site
 * fixes, the browser UI theme, and per-tab pauses. Content scripts ask it
 * for a ready-to-apply state per page.
 */
import { fixFor, parseFixes, type FixEntry } from "./shared/fixes";
import { listAllows, migrate, siteOverride } from "./shared/settings";
import { browserTheme, darkReaderTheme, pageFix, seedPalette, wantsDark } from "./shared/palette";
import type { ContentState, PaletteMessage, Settings } from "./shared/types";

const HOST = "m3_browser";
const UBLOCK_ID = "uBlock0@raymondhill.net";
const DARK_SITE_TTL = 30 * 24 * 3600 * 1000;

let settings: Settings = migrate(undefined);
let palette: PaletteMessage | null = null;
let hostConnected = false;
let port: browser.runtime.Port | null = null;
let reconnectDelay = 1000;
const pausedTabs = new Set<number>();
/** Whether each origin looked dark on its own, and when we last looked. */
let siteLooks: Record<string, { dark: boolean; at: number }> = {};
let fixes: FixEntry[] | null = null;
const wallpaperWaiters: ((url: string | null) => void)[] = [];

const systemDark = () => matchMedia("(prefers-color-scheme: dark)").matches;

async function loadFixes(): Promise<FixEntry[]> {
  if (fixes) return fixes;
  try {
    const text = await (await fetch(browser.runtime.getURL("data/dynamic-theme-fixes.config"))).text();
    fixes = parseFixes(text);
  } catch {
    fixes = [];
  }
  return fixes;
}

/** The palette in use: host > imported file > seed. */
function currentPalette(): PaletteMessage {
  return palette ?? settings.importedPalette ?? seedPalette(settings.seed, systemDark());
}

async function applyBrowserTheme() {
  if (!browser.theme) return;
  if (!settings.browserTheme) {
    await browser.theme.reset();
    return;
  }
  const p = currentPalette();
  const dark = settings.mode === "light" ? false : settings.mode === "dark" ? true : p.shellDark;
  await browser.theme.update(browserTheme(dark ? p.dark : p.light, dark));
}

/** Tell every tab (all frames) to fetch its state again. */
async function refreshTabs() {
  const tabs = await browser.tabs.query({});
  await Promise.all(tabs.map((t) => (t.id === undefined ? null : browser.tabs.sendMessage(t.id, { type: "refresh" }).catch(() => null))));
}

async function changed() {
  await applyBrowserTheme();
  await refreshTabs();
}

function connectHost() {
  try {
    port = browser.runtime.connectNative(HOST);
  } catch {
    port = null;
    return;
  }
  port.onMessage.addListener((msg: unknown) => {
    const m = msg as { type?: string; dataUrl?: string | null };
    if (m.type === "palette") {
      hostConnected = true;
      reconnectDelay = 1000;
      palette = msg as PaletteMessage;
      void browser.storage.local.set({ palette });
      void changed();
    } else if (m.type === "wallpaper") {
      const url = m.dataUrl ?? null;
      if (url) void browser.storage.local.set({ wallpaper: { dataUrl: url, at: Date.now() } });
      wallpaperWaiters.splice(0).forEach((w) => w(url));
    }
  });
  port.onDisconnect.addListener(() => {
    hostConnected = false;
    port = null;
    wallpaperWaiters.splice(0).forEach((w) => w(null));
    // The host may not be installed (e.g. inside a Flatpak); retry slowly.
    setTimeout(connectHost, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, 5 * 60 * 1000);
  });
  port.postMessage({ type: "hello" });
}

function origin(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}

async function contentState(url: string, tabId?: number): Promise<ContentState> {
  const p = currentPalette();
  const site = siteOverride(settings, url)?.override;
  const dark = wantsDark(p, settings, site);
  const off = (reason: string): ContentState => ({ enabled: false, reason, dark: !!dark, detectDark: false });
  if (dark === null) return off("off");
  if (tabId !== undefined && pausedTabs.has(tabId)) return off("paused");
  if (!listAllows(settings, url)) return off(settings.listMode === "whitelist" ? "not-whitelisted" : "blacklist");
  const o = origin(url);
  const look = o ? siteLooks[o] : undefined;
  const known = look && Date.now() - look.at < DARK_SITE_TTL ? look : undefined;
  const skipDark = dark && settings.skipDarkSites && !site?.mode;
  if (skipDark && known?.dark) return off("already-dark");
  const roles = dark ? p.dark : p.light;
  return {
    enabled: true,
    dark,
    theme: darkReaderTheme(roles, dark, settings, site),
    fixes: pageFix(fixFor(await loadFixes(), url), roles, dark, settings, site),
    detectDark: skipDark && known === undefined,
  };
}

browser.runtime.onMessage.addListener(async (msg: unknown, sender: browser.runtime.MessageSender) => {
  const m = msg as Record<string, unknown>;
  switch (m.type) {
    case "getContentState":
      return contentState(String(m.url ?? sender.url ?? ""), sender.tab?.id);
    case "fetch": {
      // Cross-origin stylesheets for Dark Reader, fetched with our host permissions.
      const r = await fetch(String(m.url), { credentials: "omit", cache: "force-cache" });
      return r.ok ? r.text() : Promise.reject(new Error(`${r.status}`));
    }
    case "reportDark": {
      const o = origin(String(m.url));
      if (o) {
        siteLooks[o] = { dark: !!m.dark, at: Date.now() };
        await browser.storage.local.set({ siteLooks });
      }
      return true;
    }
    case "getStatus": {
      const tabId = typeof m.tabId === "number" ? m.tabId : undefined;
      return {
        hostConnected,
        source: currentPalette().source,
        palette: currentPalette(),
        paused: tabId !== undefined && pausedTabs.has(tabId),
      };
    }
    case "setPaused": {
      const tabId = Number(m.tabId);
      if (m.paused) pausedTabs.add(tabId);
      else pausedTabs.delete(tabId);
      await browser.tabs.sendMessage(tabId, { type: "refresh" }).catch(() => null);
      return true;
    }
    case "themeAnyway": {
      // Remember the site as not dark, so it isn't detected and skipped again.
      const o = origin(String(m.url));
      if (o) siteLooks[o] = { dark: false, at: Date.now() };
      await browser.storage.local.set({ siteLooks });
      await refreshTabs();
      return true;
    }
    case "wallpaper": {
      if (!port) {
        const stored = (await browser.storage.local.get("wallpaper")).wallpaper as { dataUrl?: string } | undefined;
        return stored?.dataUrl ?? null;
      }
      const result = new Promise<string | null>((resolve) => wallpaperWaiters.push(resolve));
      port.postMessage({ type: "wallpaper", maxWidth: Number(m.maxWidth) || 1920 });
      return result;
    }
    case "ublock": {
      try {
        const info = await browser.management.get(UBLOCK_ID);
        return { installed: true, enabled: info.enabled, optionsUrl: info.optionsUrl ?? null };
      } catch {
        return { installed: false, enabled: false, optionsUrl: null };
      }
    }
    default:
      return undefined;
  }
});

browser.tabs.onRemoved.addListener((id) => pausedTabs.delete(id));

browser.storage.onChanged.addListener((changes, area) => {
  if (area !== "local" || !changes.settings) return;
  settings = migrate(changes.settings.newValue);
  void changed();
});

matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if (!palette) void changed();
});

async function init() {
  const stored = await browser.storage.local.get(["settings", "palette", "siteLooks"]);
  settings = migrate(stored.settings);
  if (JSON.stringify(stored.settings) !== JSON.stringify(settings)) {
    await browser.storage.local.set({ settings });
  }
  siteLooks = (stored.siteLooks as typeof siteLooks) ?? {};
  for (const [o, look] of Object.entries(siteLooks)) {
    if (Date.now() - look.at > DARK_SITE_TTL) delete siteLooks[o];
  }
  void browser.storage.local.remove("darkSites");
  // Use the last palette from the host until it answers.
  palette = (stored.palette as PaletteMessage) ?? null;
  await applyBrowserTheme();
  connectHost();
  void loadFixes();
  void replaceHomeTabs();
}

/**
 * A dev-loaded extension is installed after the first window has already
 * opened the browser's own home page, so swap untouched home tabs for ours.
 */
async function replaceHomeTabs() {
  const page = browser.runtime.getURL("newtab.html");
  for (const tab of await browser.tabs.query({})) {
    if (tab.id !== undefined && (tab.url === "about:home" || tab.url === "about:newtab")) {
      await browser.tabs.update(tab.id, { url: page, loadReplace: true }).catch(() => null);
    }
  }
}

void init();
