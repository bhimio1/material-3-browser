import { h, icon, themePage, saveSettings, loadSettings, hasBrowser } from "./ui";
import type { Settings, SpeedDialItem } from "../shared/types";

let settings: Settings;
let editing = false;

function faviconUrl(url: string): string | null {
  try {
    return `https://icons.duckduckgo.com/ip3/${new URL(url).hostname}.ico`;
  } catch {
    return null;
  }
}

function fav(url: string, title: string): HTMLElement {
  const box = h("div", { class: "fav" }, (title.trim()[0] ?? "?").toUpperCase());
  const src = faviconUrl(url);
  if (src) {
    const img = h("img", { src, alt: "" });
    img.addEventListener("load", () => box.replaceChildren(img));
  }
  return box;
}

function clock(): HTMLElement[] {
  const time = h("div", { class: "clock" });
  const date = h("div", { class: "date" });
  const tick = () => {
    const now = new Date();
    time.textContent = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: !settings.newtab.clock24 });
    date.textContent = now.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" });
  };
  tick();
  setInterval(tick, 1000);
  return [time, date];
}

function search(): HTMLElement {
  const input = h("input", { type: "search", placeholder: "Search or type an address", autofocus: true });
  input.addEventListener("keydown", async (e) => {
    if (e.key !== "Enter") return;
    const q = input.value.trim();
    if (!q) return;
    const looksLikeUrl = /^[a-z]+:\/\//i.test(q) || (/^[^\s]+\.[a-z]{2,}(\/.*)?$/i.test(q) && !q.includes(" "));
    if (looksLikeUrl) {
      location.href = /^[a-z]+:\/\//i.test(q) ? q : `https://${q}`;
    } else if (hasBrowser && browser.search) {
      const tab = await browser.tabs.getCurrent();
      await browser.search.search({ query: q, engine: settings.newtab.search || undefined, tabId: tab?.id });
    } else {
      location.href = `https://duckduckgo.com/?q=${encodeURIComponent(q)}`;
    }
  });
  return h("div", { class: "search" }, icon("search"), input);
}

function addDialog(onAdd: (d: SpeedDialItem) => void) {
  const title = h("input", { type: "text", placeholder: "Name" });
  const url = h("input", { type: "url", placeholder: "https://…" });
  const dialog = h("div", { class: "dialog" });
  const close = () => dialog.remove();
  const add = () => {
    const u = url.value.trim();
    if (!u) return;
    const full = /^[a-z]+:\/\//i.test(u) ? u : `https://${u}`;
    onAdd({ title: title.value.trim() || new URL(full).hostname, url: full });
    close();
  };
  dialog.append(h("div", { class: "sheet" }, h("h3", {}, "Add a shortcut"), title, url,
    h("div", { class: "actions" }, h("button", { onclick: close }, "Cancel"), h("button", { class: "filled", onclick: add }, "Add"))));
  dialog.addEventListener("click", (e) => e.target === dialog && close());
  document.body.append(dialog);
  title.focus();
}

function speedDial(): HTMLElement {
  const items = settings.newtab.speedDial;
  const save = async (next: SpeedDialItem[]) => {
    settings = { ...settings, newtab: { ...settings.newtab, speedDial: next } };
    await saveSettings(settings);
    void render();
  };
  const grid = h("div", { class: `dial${editing ? " editing" : ""}` },
    ...items.map((d, i) => h("a", { class: "tile", href: d.url, title: d.url },
      fav(d.url, d.title), h("span", { class: "name" }, d.title),
      h("button", { class: "icon-button remove", title: "Remove", onclick: (e: Event) => {
        e.preventDefault();
        void save(items.filter((_, j) => j !== i));
      } }, icon("close")))),
  );
  if (editing) grid.append(h("div", { class: "tile add", onclick: () => addDialog((d) => void save([...items, d])) }, h("div", { class: "fav" }, icon("add")), h("span", { class: "name" }, "Add")));
  return grid;
}

async function recent(): Promise<HTMLElement | null> {
  if (!settings.newtab.showRecent || !hasBrowser || !browser.sessions) return null;
  const sessions = (await browser.sessions.getRecentlyClosed({ maxResults: 5 })).filter((s) => s.tab?.url?.startsWith("http"));
  if (!sessions.length) return null;
  return h("div", { class: "recent" }, h("h2", {}, "Recently closed"),
    h("div", { class: "card" }, ...sessions.map((s) => h("a", { onclick: () => s.tab?.sessionId && browser.sessions.restore(s.tab.sessionId) },
      s.tab?.favIconUrl ? h("img", { src: s.tab.favIconUrl, alt: "" }) : icon("public"),
      h("span", { class: "title" }, s.tab?.title ?? s.tab?.url ?? ""),
      icon("history")))));
}

async function wallpaper() {
  const el = document.getElementById("wallpaper")!;
  if (!settings.newtab.showWallpaper) {
    el.classList.remove("show");
    return;
  }
  const params = new URLSearchParams(location.search);
  let url: string | null = params.get("wallpaper");
  if (!url && hasBrowser) {
    const cached = (await browser.storage.local.get("wallpaper")).wallpaper as { dataUrl?: string; at?: number } | undefined;
    url = cached?.dataUrl ?? null;
    // Refresh from the host at most every 10 minutes; it may have changed.
    if (!cached?.at || Date.now() - cached.at > 10 * 60 * 1000) {
      const fresh = (await browser.runtime.sendMessage({ type: "wallpaper", maxWidth: Math.min(3840, screen.width * devicePixelRatio) })) as string | null;
      url = fresh ?? url;
    }
  }
  if (url) {
    el.style.backgroundImage = `url("${url}")`;
    el.classList.add("show");
  }
}

async function render() {
  const app = document.getElementById("app")!;
  const recentEl = await recent();
  app.replaceChildren(
    h("div", { class: "toolbar" },
      h("button", { class: "icon-button", title: editing ? "Done" : "Edit shortcuts", onclick: () => { editing = !editing; void render(); } }, icon(editing ? "check" : "edit")),
      h("button", { class: "icon-button", title: "Settings", onclick: () => hasBrowser && browser.runtime.openOptionsPage() }, icon("settings"))),
    ...clock(), search(), speedDial(), ...(recentEl ? [recentEl] : []),
  );
  void wallpaper();
}

void themePage(async (s) => {
  settings = s;
  void wallpaper();
}).then(async () => {
  settings = await loadSettings();
  await render();
});
