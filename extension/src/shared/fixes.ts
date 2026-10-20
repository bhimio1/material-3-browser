/**
 * Dark Reader's per-site fixes (dynamic-theme-fixes.config, MIT, Dark
 * Reader Ltd.). Format: blocks separated by `====…` lines; each block starts
 * with URL patterns, followed by INVERT / CSS / IGNORE INLINE STYLE /
 * IGNORE IMAGE ANALYSIS sections. The `*` block applies everywhere.
 */
import type { Fix } from "./types";
import { matches } from "./match";

export interface FixEntry extends Fix {
  url: string[];
}

const SECTIONS: Record<string, keyof Fix> = {
  INVERT: "invert",
  CSS: "css",
  "IGNORE INLINE STYLE": "ignoreInlineStyle",
  "IGNORE IMAGE ANALYSIS": "ignoreImageAnalysis",
};

export function parseFixes(text: string): FixEntry[] {
  const out: FixEntry[] = [];
  for (const block of text.split(/^=+\s*$/m)) {
    const lines = block.split("\n");
    const entry: FixEntry = { url: [], invert: [], css: "", ignoreInlineStyle: [], ignoreImageAnalysis: [] };
    let section: keyof Fix | null = null;
    const css: string[] = [];
    for (const raw of lines) {
      const line = raw.replace(/\s+$/, "");
      const name = SECTIONS[line.trim()];
      if (name) {
        section = name;
        continue;
      }
      if (section === null) {
        if (line.trim()) entry.url.push(line.trim());
      } else if (section === "css") {
        css.push(raw);
      } else if (line.trim()) {
        (entry[section] as string[]).push(line.trim());
      }
    }
    entry.css = css.join("\n").trim();
    // Some sections (like DISABLE…) we don't use: skip URL-only blocks too.
    if (entry.url.length && (entry.invert.length || entry.css || entry.ignoreInlineStyle.length || entry.ignoreImageAnalysis.length)) {
      out.push(entry);
    }
  }
  return out;
}

/** The generic fix merged with the most specific matching site fix. */
export function fixFor(entries: FixEntry[], url: string): Fix | undefined {
  const generic = entries.find((e) => e.url.includes("*"));
  let best: FixEntry | undefined;
  let bestLen = -1;
  for (const e of entries) {
    if (e === generic) continue;
    for (const p of e.url) {
      if (p.length > bestLen && matches(p, url)) {
        best = e;
        bestLen = p.length;
      }
    }
  }
  if (!generic && !best) return undefined;
  const parts = [generic, best].filter((x): x is FixEntry => !!x);
  return {
    invert: parts.flatMap((p) => p.invert),
    css: parts.map((p) => p.css).filter(Boolean).join("\n"),
    ignoreInlineStyle: parts.flatMap((p) => p.ignoreInlineStyle),
    ignoreImageAnalysis: parts.flatMap((p) => p.ignoreImageAnalysis),
  };
}
