/**
 * Content script (all frames, document_start): applies Dark Reader's
 * dynamic theme with Material 3 colors, or leaves the page alone.
 */
// Must come before "darkreader", which replaces runtime.sendMessage.
import { sendMessage } from "./shared/runtime";
import * as DarkReader from "darkreader";
import type { ContentState } from "./shared/types";
import { pageIsDark } from "./shared/detect";

let applied = "";

DarkReader.setFetchMethod(async (url: string) => {
  const text = (await sendMessage({ type: "fetch", url })) as string;
  return new Response(text, { headers: { "Content-Type": "text/css" } });
});

function domReady(): Promise<void> {
  return document.readyState === "loading"
    ? new Promise((r) => document.addEventListener("DOMContentLoaded", () => r(), { once: true }))
    : Promise.resolve();
}

/** The page's own stylesheets, so a dark site isn't judged by its unstyled body. */
function stylesLoaded(timeout = 1500): Promise<void> {
  return document.readyState === "complete"
    ? Promise.resolve()
    : new Promise((r) => {
        window.addEventListener("load", () => r(), { once: true });
        setTimeout(r, timeout);
      });
}

async function apply() {
  let state: ContentState;
  try {
    state = (await sendMessage({ type: "getContentState", url: location.href })) as ContentState;
  } catch {
    return;
  }
  if (!state?.enabled || !state.theme) {
    if (DarkReader.isEnabled()) DarkReader.disable();
    applied = "";
    return;
  }
  if (state.detectDark) {
    // First visit with "skip already-dark sites": look before theming.
    await domReady();
    let dark = pageIsDark(document);
    if (!dark) {
      await stylesLoaded();
      dark = pageIsDark(document);
    }
    void sendMessage({ type: "reportDark", url: location.href, dark }).catch(() => null);
    if (dark) return;
  }
  const key = JSON.stringify([state.theme, state.fixes]);
  if (key === applied) return;
  applied = key;
  DarkReader.enable(state.theme as Partial<DarkReader.Theme>, state.fixes as Partial<DarkReader.DynamicThemeFix> as DarkReader.DynamicThemeFix);
}

browser.runtime.onMessage.addListener((msg: unknown) => {
  if ((msg as { type?: string }).type === "refresh") void apply();
});

if (location.protocol === "http:" || location.protocol === "https:" || location.protocol === "file:") {
  void apply();
}
