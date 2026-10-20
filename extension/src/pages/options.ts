import { h, icon, row, segmented, slider, switchInput, themePage, saveSettings, loadSettings, toast, hasBrowser, loadPalette } from "./ui";
import { migrate } from "../shared/settings";
import { paletteFromColorsJson } from "../shared/palette";
import type { Settings, SiteOverride } from "../shared/types";

const SECTIONS: [string, string, string][] = [
  ["palette", "palette", "Palette"],
  ["theming", "format_color_fill", "Website theming"],
  ["sites", "block", "Sites"],
  ["newtab", "tab", "New tab"],
  ["adblock", "shield", "Ad blocking"],
  ["backup", "download", "Backup"],
];

let settings: Settings;

async function save(next: Settings, rerender = true) {
  settings = migrate(next);
  await saveSettings(settings);
  if (rerender) void render();
}

function section(id: string, title: string, subtitle: string, ...cards: Node[]): HTMLElement {
  return h("section", { id }, h("h1", {}, title), subtitle ? h("div", { class: "subtitle" }, subtitle) : null, ...cards);
}

function card(title: string, ...rows: (Node | null)[]): DocumentFragment {
  const f = document.createDocumentFragment();
  if (title) f.append(h("h2", {}, title));
  f.append(h("div", { class: "card" }, ...rows));
  return f;
}

function listEditor(list: string[], placeholder: string, onChange: (l: string[]) => void): HTMLElement {
  const input = h("input", { type: "text", placeholder });
  const add = () => {
    const v = input.value.trim();
    if (v && !list.includes(v)) onChange([...list, v]);
  };
  input.addEventListener("keydown", (e) => e.key === "Enter" && add());
  return h("div", {},
    h("div", { class: "chips" }, ...(list.length
      ? list.map((p) => h("span", { class: "chip" }, p, h("button", { class: "icon-button", title: "Remove", onclick: () => onChange(list.filter((x) => x !== p)) }, icon("close"))))
      : [h("span", { class: "desc" }, "Nothing here yet.")])),
    h("div", { class: "add-row" }, input, h("button", { class: "tonal", onclick: add }, icon("add"), "Add")));
}

function overrideEditor(pattern: string, o: SiteOverride): HTMLElement {
  const update = (patch: Partial<SiteOverride>) => save({ ...settings, sites: { ...settings.sites, [pattern]: { ...o, ...patch } } }, false);
  const num = (key: "brightness" | "contrast" | "sepia" | "grayscale", max: number) =>
    h("label", {}, key[0].toUpperCase() + key.slice(1), ...slider(o[key] ?? settings[key], 0, max, (v) => update({ [key]: v })));
  return h("div", { class: "override" },
    h("div", { class: "head" },
      h("span", { class: "pattern" }, pattern),
      segmented((o.mode ?? "auto") as string, [["auto", "Global"], ["dark", "Dark"], ["light", "Light"], ["off", "Off"]], (m) =>
        save({ ...settings, sites: { ...settings.sites, [pattern]: { ...o, mode: m === "auto" ? undefined : (m as SiteOverride["mode"]) } } })),
      h("button", { class: "icon-button", title: "Remove", onclick: () => {
        const sites = { ...settings.sites };
        delete sites[pattern];
        void save({ ...settings, sites });
      } }, icon("delete"))),
    h("div", { class: "grid" },
      num("brightness", 150), num("contrast", 150), num("sepia", 100), num("grayscale", 100),
      h("label", {}, "Background", h("input", { type: "color", value: o.background ?? "#000000", onchange: (e: Event) => update({ background: (e.target as HTMLInputElement).value }) })),
      h("label", {}, "Text", h("input", { type: "color", value: o.text ?? "#ffffff", onchange: (e: Event) => update({ text: (e.target as HTMLInputElement).value }) })),
    ),
    h("details", {}, h("summary", { class: "desc" }, "Custom CSS"),
      h("textarea", { placeholder: "/* Extra CSS for this site. ${color} wraps a color so it gets themed too. */", value: o.css ?? "",
        onchange: (e: Event) => update({ css: (e.target as HTMLTextAreaElement).value }) })),
  );
}

async function render() {
  const app = document.getElementById("app")!;
  settings = await loadSettings();
  const palette = await loadPalette(settings);
  const status = hasBrowser ? (await browser.runtime.sendMessage({ type: "getStatus" })) as { hostConnected: boolean; source: string } : { hostConnected: true, source: palette.source };
  const s = settings;

  // ---- Palette
  const roles = (palette.shellDark ? palette.dark : palette.light);
  const swatches = h("div", { class: "swatches" }, ...["primary", "secondary", "tertiary", "primary_container", "secondary_container", "surface_container_high", "surface"].map((r) =>
    h("span", { class: "swatch", title: r, style: `background:${roles[r]}` })));
  const importInput = h("input", { type: "file", accept: ".json,application/json", style: "display:none", onchange: async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const p = paletteFromColorsJson(JSON.parse(await file.text()));
      if (!p) throw new Error("no colors");
      await save({ ...s, importedPalette: p });
      toast("Palette imported");
    } catch {
      toast("That file isn't a matugen / end-4 colors.json");
    }
  } });
  const sourceText = status.hostConnected
    ? "Live from your desktop: end4-pC's colors (or m3-browser's seed color), updated whenever the wallpaper changes."
    : s.importedPalette ? "Using an imported colors.json. The palette host isn't connected." : "Using the seed color below. The palette host isn't connected (see m3-browser --settings).";
  const paletteSection = section("palette", "Palette", "Every color here comes from your Material 3 palette.",
    card("Source",
      row(status.hostConnected ? "Connected to m3-browser" : "Not connected", sourceText, swatches),
      row("Seed color", "Used when neither the desktop palette nor an imported file is available.",
        h("input", { type: "color", value: s.seed, onchange: (e: Event) => save({ ...s, seed: (e.target as HTMLInputElement).value }) })),
      row("Import colors.json", "For setups where the palette host can't run, such as the Zen Flatpak without host access.",
        importInput,
        s.importedPalette ? h("button", { class: "outlined", onclick: () => save({ ...s, importedPalette: null }) }, "Clear") : null,
        h("button", { class: "tonal", onclick: () => importInput.click() }, icon("upload"), "Import")),
    ),
    card("Browser",
      row("Theme the browser", "Paint tabs, toolbars, menus and Zen's sidebar with the palette.", switchInput(s.browserTheme, (v) => save({ ...s, browserTheme: v }))),
    ),
  );

  // ---- Theming
  const theming = section("theming", "Website theming", "How websites are recolored. Per-site settings are under Sites.",
    card("Mode",
      row("Theme websites", "Auto follows your desktop's light or dark mode.",
        segmented(s.mode, [["follow", "Auto"], ["dark", "Dark"], ["light", "Light"], ["off", "Off"]], (m) => save({ ...s, mode: m }))),
      row("Leave dark sites alone", "Sites that are already dark keep their own colors in dark mode.", switchInput(s.skipDarkSites, (v) => save({ ...s, skipDarkSites: v }))),
    ),
    card("Strength",
      row("Brightness", "", ...slider(s.brightness, 50, 150, (v) => save({ ...s, brightness: v }, false))),
      row("Contrast", "", ...slider(s.contrast, 50, 150, (v) => save({ ...s, contrast: v }, false))),
      row("Sepia", "", ...slider(s.sepia, 0, 100, (v) => save({ ...s, sepia: v }, false))),
      row("Grayscale", "", ...slider(s.grayscale, 0, 100, (v) => save({ ...s, grayscale: v }, false))),
    ),
    card("Details",
      row("Material accents", "Primary color for text links, focus rings, checkboxes and the text cursor.", switchInput(s.accents, (v) => save({ ...s, accents: v }))),
      row("Dim images", "In dark mode only.", ...slider(s.dimImages, 0, 50, (v) => save({ ...s, dimImages: v }, false))),
      row("Leave videos alone", "Don't analyse or recolor video players.", switchInput(s.excludeVideo, (v) => save({ ...s, excludeVideo: v }))),
      row("Themed scrollbars", "", switchInput(s.scrollbars, (v) => save({ ...s, scrollbars: v }))),
      row("Font", "Replace every site's font.",
        h("input", { type: "text", value: s.font.family, style: "width:160px", onchange: (e: Event) => save({ ...s, font: { ...s.font, family: (e.target as HTMLInputElement).value } }) }),
        switchInput(s.font.enabled, (v) => save({ ...s, font: { ...s.font, enabled: v } }))),
    ),
  );

  // ---- Sites
  const newOverride = h("input", { type: "text", placeholder: "example.com or github.com/*/settings" });
  const addOverride = () => {
    const p = newOverride.value.trim();
    if (p && !s.sites[p]) void save({ ...s, sites: { ...s.sites, [p]: {} } });
  };
  newOverride.addEventListener("keydown", (e) => e.key === "Enter" && addOverride());
  const sites = section("sites", "Sites", "Patterns: example.com (and its subdomains), *.example.com, google.* (any domain ending), github.com/*/settings (a path), or /regex/.",
    card("Which sites",
      row("List mode", s.listMode === "whitelist" ? "Only the sites on the whitelist are themed." : "Every site is themed except the blacklist.",
        segmented(s.listMode, [["blacklist", "Blacklist"], ["whitelist", "Whitelist"]], (m) => save({ ...s, listMode: m }))),
      s.listMode === "blacklist"
        ? listEditor(s.blacklist, "Never theme…", (l) => save({ ...s, blacklist: l }))
        : listEditor(s.whitelist, "Theme only…", (l) => save({ ...s, whitelist: l })),
    ),
    card("Per-site settings",
      ...Object.entries(s.sites).map(([p, o]) => overrideEditor(p, o)),
      h("div", { class: "add-row" }, newOverride, h("button", { class: "tonal", onclick: addOverride }, icon("add"), "Add site")),
    ),
  );

  // ---- New tab
  const engines = hasBrowser && browser.search ? await browser.search.get() : [];
  const engineSelect = h("select", { onchange: (e: Event) => save({ ...s, newtab: { ...s.newtab, search: (e.target as HTMLSelectElement).value } }) },
    h("option", { value: "" }, "Browser default"),
    ...engines.map((e) => h("option", { value: e.name, selected: e.name === s.newtab.search }, e.name)));
  const dial = s.newtab.speedDial;
  const setDial = (d: typeof dial) => save({ ...s, newtab: { ...s.newtab, speedDial: d } });
  const newtab = section("newtab", "New tab", "",
    card("Page",
      row("Search engine", "", engineSelect),
      row("Wallpaper", "Show your end4-pC wallpaper behind the page.", switchInput(s.newtab.showWallpaper, (v) => save({ ...s, newtab: { ...s.newtab, showWallpaper: v } }))),
      row("24-hour clock", "", switchInput(s.newtab.clock24, (v) => save({ ...s, newtab: { ...s.newtab, clock24: v } }))),
      row("Recently closed tabs", "", switchInput(s.newtab.showRecent, (v) => save({ ...s, newtab: { ...s.newtab, showRecent: v } }))),
    ),
    card("Speed dial",
      ...dial.map((d, i) => h("div", { class: "dial-item" },
        h("input", { type: "text", value: d.title, onchange: (e: Event) => setDial(dial.map((x, j) => (j === i ? { ...x, title: (e.target as HTMLInputElement).value } : x))) }),
        h("input", { type: "url", value: d.url, onchange: (e: Event) => setDial(dial.map((x, j) => (j === i ? { ...x, url: (e.target as HTMLInputElement).value } : x))) }),
        h("button", { class: "icon-button", title: "Remove", onclick: () => setDial(dial.filter((_, j) => j !== i)) }, icon("delete")))),
      h("div", { class: "add-row" }, h("button", { class: "tonal", onclick: () => setDial([...dial, { title: "New site", url: "https://" }]) }, icon("add"), "Add")),
    ),
  );

  // ---- Ad blocking
  const ublock = hasBrowser ? (await browser.runtime.sendMessage({ type: "ublock" })) as { installed: boolean; enabled: boolean; optionsUrl: string | null } : { installed: true, enabled: true, optionsUrl: null };
  const adblock = section("adblock", "Ad blocking", "uBlock Origin is installed by m3-browser and keeps itself updated.",
    card("",
      row("uBlock Origin", !ublock.installed ? "Not installed. Turn it on in m3-browser --settings." : ublock.enabled ? "Blocking ads, trackers and malware domains." : "Installed but turned off in about:addons.",
        ublock.optionsUrl ? h("button", { class: "tonal", onclick: () => browser.tabs.create({ url: ublock.optionsUrl! }) }, icon("open_in_new"), "Dashboard") : null),
      row("Allow a site", "Click uBlock Origin's toolbar button on that site and turn off the big power switch.", null),
    ),
  );

  // ---- Backup
  const backupInput = h("input", { type: "file", accept: ".json,application/json", style: "display:none", onchange: async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      await save(migrate(JSON.parse(await file.text())));
      toast("Settings imported");
    } catch {
      toast("That file isn't a settings export");
    }
  } });
  const backup = section("backup", "Backup", "Settings, lists and per-site options as a JSON file.",
    card("",
      row("Export", "", h("button", { class: "tonal", onclick: () => {
        const blob = new Blob([JSON.stringify(settings, null, 2)], { type: "application/json" });
        const a = h("a", { href: URL.createObjectURL(blob), download: "material-3-browser-settings.json" });
        a.click();
      } }, icon("download"), "Export")),
      row("Import", "Replaces your current settings.", backupInput, h("button", { class: "tonal", onclick: () => backupInput.click() }, icon("upload"), "Import")),
      row("Reset", "Back to defaults.", h("button", { class: "outlined", onclick: () => confirm("Reset all Material 3 Browser settings?") && save(migrate(undefined)) }, "Reset")),
    ),
  );

  const scrollTop = window.scrollY;
  app.replaceChildren(paletteSection, theming, sites, newtab, adblock, backup);
  window.scrollTo(0, scrollTop);
}

function renderNav() {
  const nav = document.getElementById("nav")!;
  nav.replaceChildren(
    h("div", { class: "brand" }, icon("palette"), "Material 3 Browser"),
    ...SECTIONS.map(([id, ic, label]) => h("a", { href: `#${id}`, "data-id": id }, icon(ic), label)),
  );
  const update = () => {
    const current = SECTIONS.map(([id]) => document.getElementById(id)).filter((e): e is HTMLElement => !!e)
      .filter((e) => e.getBoundingClientRect().top < 160).pop()?.id ?? SECTIONS[0][0];
    nav.querySelectorAll("a").forEach((a) => a.classList.toggle("active", a.dataset.id === current));
  };
  window.addEventListener("scroll", update, { passive: true });
  setTimeout(update, 50);
}

renderNav();
void themePage().then(render);
