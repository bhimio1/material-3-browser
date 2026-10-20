// Bundles the extension into dist/ (unpacked) with esbuild.
import { build } from "esbuild";
import { cpSync, mkdirSync, rmSync, readFileSync, writeFileSync } from "node:fs";

const watch = process.argv.includes("--watch");
rmSync("dist", { recursive: true, force: true });
mkdirSync("dist", { recursive: true });

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const manifest = JSON.parse(readFileSync("static/manifest.json", "utf8"));
manifest.version = pkg.version;
writeFileSync("dist/manifest.json", JSON.stringify(manifest, null, 2));

cpSync("static", "dist", { recursive: true, filter: (src) => !src.endsWith("manifest.json") });
cpSync("data", "dist/data", { recursive: true });
cpSync("LICENSE-THIRD-PARTY.md", "dist/LICENSE-THIRD-PARTY.md");

await build({
  entryPoints: {
    background: "src/background.ts",
    content: "src/content.ts",
    popup: "src/pages/popup.ts",
    options: "src/pages/options.ts",
    newtab: "src/pages/newtab.ts",
  },
  outdir: "dist",
  bundle: true,
  format: "iife",
  target: "firefox140",
  minify: !watch,
  sourcemap: watch ? "inline" : false,
  legalComments: "none",
  logLevel: "info",
});
