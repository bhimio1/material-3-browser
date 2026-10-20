// Renders the fixture pages before/after theming, and the extension's pages,
// in headless Chromium. Usage: node test/visual/render.mjs [outdir]
import { build } from "esbuild";
import { chromium } from "playwright-core";
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const out = resolve(process.argv[2] ?? `${root}/test/visual/out`);
mkdirSync(out, { recursive: true });

const common = { bundle: true, logLevel: "warning", absWorkingDir: root };
await build({ ...common, entryPoints: ["test/visual/harness.ts"], outfile: `${out}/harness.js`, format: "iife" });
await build({ ...common, entryPoints: ["test/visual/gen-palette.ts"], outfile: `${out}/gen-palette.cjs`, platform: "node", format: "cjs" });
execFileSync("node", [`${out}/gen-palette.cjs`, "#7A5AB8", `${out}/colors.json`, `${out}/palette.json`]);
execFileSync("node", [`${out}/gen-palette.cjs`, "#2E7D6B", `${out}/colors-teal.json`, `${out}/palette-teal.json`]);
const colors = JSON.parse(readFileSync(`${out}/colors.json`, "utf8"));

const server = spawn("python3", ["-m", "http.server", "8765", "--bind", "127.0.0.1"], { cwd: root, stdio: "ignore" });
await new Promise((r) => setTimeout(r, 800));
const base = "http://127.0.0.1:8765";
const executablePath = process.env.CHROMIUM ?? "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const browser = await chromium.launch({ executablePath });

try {
  const ctx = await browser.newContext({ viewport: { width: 900, height: 640 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const outOf = (p) => out.replace(root + "/", "") + "/" + p;
  for (const [name, opts] of [["article", {}], ["dark", {}], ["form", {}], ["images", { settings: { dimImages: 15 } }], ["article-light", { dark: false }]]) {
    const file = name.replace("-light", "");
    await page.goto(`${base}/test/visual/fixtures/${file}.html`);
    await page.screenshot({ path: `${out}/${name}-before.png` });
    await page.addScriptTag({ url: `${base}/${outOf("harness.js")}` });
    const result = await page.evaluate(([c, o]) => window.m3Theme(c, o), [colors, opts]);
    await page.waitForTimeout(700);
    await page.screenshot({ path: `${out}/${name}-after.png` });
    console.log(name, result);
    execFileSync("convert", [`${out}/${name}-before.png`, `${out}/${name}-after.png`, "-background", "#888", "-splice", "8x0", "+append", "-chop", "8x0", `${out}/${name}-compare.png`]);
  }

  // The extension's own pages, outside the browser, with the sample palette.
  execFileSync("convert", ["-size", "1600x1000", "gradient:#3b2a6b-#c26fa0", "-blur", "0x8", `${out}/wallpaper.jpg`]);
  const pages = [
    ["options", "dist/options.html", { width: 1180, height: 820 }],
    ["newtab", "dist/newtab.html", { width: 1180, height: 820 }],
    ["popup", "dist/popup.html", { width: 360, height: 560 }],
  ];
  for (const [name, path, viewport] of pages) {
    const p2 = await ctx.newPage();
    await p2.setViewportSize(viewport);
    await p2.goto(`${base}/${path}?palette=/${outOf("palette.json")}&wallpaper=/${outOf("wallpaper.jpg")}`);
    await p2.waitForTimeout(800);
    await p2.screenshot({ path: `${out}/${name}.png`, fullPage: name === "options" });
    console.log("page", name);
  }
} finally {
  await browser.close();
  server.kill();
}
