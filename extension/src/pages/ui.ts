/** Shared helpers for the extension's pages: palette → CSS, DOM, icons. */
import { migrate } from "../shared/settings";
import { seedPalette } from "../shared/palette";
import type { PaletteMessage, Roles, Settings } from "../shared/types";

import CODEPOINTS from "./icons.json";

/** A Material Symbols icon (the same font the Rust apps use). */
export function icon(name: string, cls = "icon"): HTMLElement {
  const span = document.createElement("span");
  span.className = cls;
  span.setAttribute("aria-hidden", "true");
  const cp = (CODEPOINTS as Record<string, string>)[name];
  span.textContent = cp ? String.fromCodePoint(parseInt(cp, 16)) : "";
  return span;
}

type Child = Node | string | null | undefined | false;
export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, unknown> = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === null || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v as EventListener);
    else if (k === "class") el.className = String(v);
    else if (k in el && typeof v !== "string") (el as unknown as Record<string, unknown>)[k] = v;
    else el.setAttribute(k, String(v));
  }
  for (const c of children) if (c !== null && c !== undefined && c !== false) el.append(c);
  return el;
}

/** A settings row: title + description on the left, control on the right. */
export function row(title: string, desc: string, ...control: Child[]): HTMLElement {
  return h("div", { class: "row" },
    h("div", { class: "text" }, h("div", { class: "title" }, title), desc ? h("div", { class: "desc" }, desc) : null),
    h("div", { class: "control" }, ...control));
}

export function switchInput(checked: boolean, onChange: (v: boolean) => void): HTMLInputElement {
  return h("input", { type: "checkbox", class: "switch", checked, onchange: (e: Event) => onChange((e.target as HTMLInputElement).checked) });
}

export function segmented<T extends string>(value: T, options: [T, string][], onChange: (v: T) => void): HTMLElement {
  const group = h("div", { class: "segmented" });
  for (const [v, label] of options) {
    group.append(h("button", { class: v === value ? "selected" : "", onclick: () => onChange(v) }, label));
  }
  return group;
}

export function slider(value: number, min: number, max: number, onChange: (v: number) => void, suffix = "%"): HTMLElement[] {
  const out = h("span", { class: "slider-value" }, `${value}${suffix}`);
  const input = h("input", {
    type: "range", min, max, step: 1, value,
    oninput: (e: Event) => { out.textContent = `${(e.target as HTMLInputElement).value}${suffix}`; },
    onchange: (e: Event) => onChange(Number((e.target as HTMLInputElement).value)),
  });
  return [input, out];
}

let toastTimer = 0;
export function toast(text: string) {
  let el = document.querySelector<HTMLElement>(".toast");
  if (!el) {
    el = h("div", { class: "toast" });
    document.body.append(el);
  }
  el.textContent = text;
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => el?.classList.remove("show"), 2500);
}

// ------------------------------------------------------------- palette

const hasBrowser = typeof browser !== "undefined" && !!browser.storage;
const systemDark = () => matchMedia("(prefers-color-scheme: dark)").matches;

export async function loadSettings(): Promise<Settings> {
  if (!hasBrowser) return migrate(undefined);
  return migrate((await browser.storage.local.get("settings")).settings);
}

export async function saveSettings(s: Settings) {
  if (hasBrowser) await browser.storage.local.set({ settings: s });
}

export async function loadPalette(settings: Settings): Promise<PaletteMessage> {
  // Pages rendered outside the browser (screenshots) can pass ?palette=<json url>.
  const params = new URLSearchParams(location.search);
  const external = params.get("palette");
  if (external) {
    try {
      return (await (await fetch(external)).json()) as PaletteMessage;
    } catch {
      /* fall through */
    }
  }
  if (hasBrowser) {
    const stored = (await browser.storage.local.get("palette")).palette as PaletteMessage | undefined;
    if (stored) return stored;
  }
  return settings.importedPalette ?? seedPalette(settings.seed, systemDark());
}

export function pickRoles(p: PaletteMessage, s: Settings): { roles: Roles; dark: boolean } {
  const dark = s.mode === "dark" ? true : s.mode === "light" ? false : p.shellDark;
  return { roles: dark ? p.dark : p.light, dark };
}

export function applyPalette(p: PaletteMessage, s: Settings) {
  const { roles, dark } = pickRoles(p, s);
  const root = document.documentElement;
  for (const [k, v] of Object.entries(roles)) root.style.setProperty(`--m3-${k.replace(/_/g, "-")}`, v);
  root.classList.toggle("light", !dark);
}

/** Load settings + palette, apply, and re-apply on changes. */
export async function themePage(onChange?: (s: Settings, p: PaletteMessage) => void): Promise<{ settings: Settings; palette: PaletteMessage }> {
  const settings = await loadSettings();
  const palette = await loadPalette(settings);
  applyPalette(palette, settings);
  if (hasBrowser) {
    browser.storage.onChanged.addListener(async (changes, area) => {
      if (area !== "local" || (!changes.settings && !changes.palette)) return;
      const s = await loadSettings();
      const p = await loadPalette(s);
      applyPalette(p, s);
      onChange?.(s, p);
    });
  }
  return { settings, palette };
}

export { hasBrowser };
