import { h, icon, row, segmented, slider, switchInput, themePage, saveSettings, loadSettings, hasBrowser } from "./ui";
import { listAllows, setSiteThemed } from "../shared/settings";
import { siteOf } from "../shared/match";
import type { Settings } from "../shared/types";

const REASONS: Record<string, string> = {
  off: "Theming is off",
  paused: "Paused in this tab",
  blacklist: "This site is on your blacklist",
  "not-whitelisted": "Not on your whitelist",
  "already-dark": "Left alone: this site is already dark",
};

async function render() {
  const app = document.getElementById("app")!;
  const settings = await loadSettings();
  const [tab] = hasBrowser ? await browser.tabs.query({ active: true, currentWindow: true }) : [{ id: 0, url: "https://example.com/" } as browser.tabs.Tab];
  const url = tab?.url ?? "";
  const site = siteOf(url);
  const status = hasBrowser ? await browser.runtime.sendMessage({ type: "getStatus", tabId: tab?.id }) as { paused: boolean; hostConnected: boolean; source: string } : { paused: false, hostConnected: true, source: "end4" };
  const state = hasBrowser && site ? await browser.runtime.sendMessage({ type: "getContentState", url }) as { enabled: boolean; reason?: string } : { enabled: true };
  const ublock = hasBrowser ? await browser.runtime.sendMessage({ type: "ublock" }) as { installed: boolean; enabled: boolean; optionsUrl: string | null } : { installed: true, enabled: true, optionsUrl: null };

  const save = async (next: Settings) => {
    await saveSettings(next);
    void render();
  };

  const favicon = h("div", { class: "favicon" }, tab?.favIconUrl ? h("img", { src: tab.favIconUrl, alt: "" }) : icon("public"));
  const reason = status.paused ? REASONS.paused : !state.enabled ? REASONS[state.reason ?? "off"] ?? "Not themed" : `Themed with your ${status.source === "end4" ? "desktop" : status.source} palette`;

  const children: (Node | null)[] = [
    h("div", { class: "site" }, favicon, h("div", {}, h("div", { class: "host" }, site ?? "This page"), h("div", { class: "status" }, site ? reason : "Built-in pages aren't themed"))),
  ];

  if (site) {
    const allowed = listAllows(settings, url);
    children.push(h("div", { class: "card" },
      row("Theme this site", settings.listMode === "whitelist" ? "Adds it to your whitelist." : "Off adds it to your blacklist.",
        switchInput(allowed, (v) => save(setSiteThemed(settings, site, v)))),
      row("Pause in this tab", "Until the tab closes.",
        switchInput(status.paused, async (v) => {
          await browser.runtime.sendMessage({ type: "setPaused", tabId: tab?.id, paused: v });
          void render();
        })),
      state.reason === "already-dark"
        ? row("Theme it anyway", "Forget that this site looked dark.", h("button", { class: "tonal", onclick: async () => {
            await browser.runtime.sendMessage({ type: "forgetDarkSite", url });
            void save(settings);
          } }, "Theme"))
        : null,
    ));
  }

  children.push(h("div", { class: "card mode" },
    segmented(settings.mode, [["follow", "Auto"], ["dark", "Dark"], ["light", "Light"], ["off", "Off"]], (m) => save({ ...settings, mode: m })),
    row("Brightness", "", ...slider(settings.brightness, 50, 150, (v) => save({ ...settings, brightness: v }))),
    row("Contrast", "", ...slider(settings.contrast, 50, 150, (v) => save({ ...settings, contrast: v }))),
  ));

  children.push(h("div", { class: "card" },
    row("uBlock Origin", !ublock.installed ? "Not installed" : ublock.enabled ? "Blocking ads and trackers" : "Turned off",
      ublock.optionsUrl ? h("button", { class: "icon-button", title: "Open its dashboard", onclick: () => browser.tabs.create({ url: ublock.optionsUrl! }) }, icon("shield")) : null),
  ));

  children.push(h("div", { class: "footer" },
    h("span", { class: "status" }, status.hostConnected ? "Live palette from your desktop" : "Palette host not connected"),
    h("button", { class: "tonal", onclick: () => (hasBrowser ? browser.runtime.openOptionsPage() : null) }, icon("settings"), "Settings"),
  ));

  app.replaceChildren(...children.filter((c): c is Node => c !== null));
}

void themePage(() => void render()).then(render);
