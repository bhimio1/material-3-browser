/** Detecting pages that are already dark, before theming them. */
import { luminance } from "./palette";

function parseColor(c: string): [number, number, number, number] | null {
  const m = c.match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const [r, g, b, a = "1"] = m[1].split(/[\s,/]+/).filter(Boolean);
  return [Number(r), Number(g), Number(b), Number(a)];
}

/** Is the page dark on its own (before we touch it)? */
export function pageIsDark(doc: Document): boolean {
  const meta = doc.querySelector('meta[name="color-scheme"]')?.getAttribute("content") ?? "";
  if (/^\s*dark\s*$/.test(meta)) return true;
  for (const el of [doc.body, doc.documentElement]) {
    if (!el) continue;
    const c = parseColor(getComputedStyle(el).backgroundColor);
    if (c && c[3] > 0.5) {
      const hex = "#" + c.slice(0, 3).map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
      return luminance(hex) < 0.2;
    }
  }
  return false;
}

