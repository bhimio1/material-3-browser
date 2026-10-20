// Builds dist/ and stages what m3-browser looks for next to itself:
//   stage/extension/              unpacked (for development loading)
//   stage/material-3-browser.xpi  packed (replaced by the signed one in CI)
import { execFileSync } from "node:child_process";
import { cpSync, rmSync, mkdirSync } from "node:fs";

execFileSync("node", ["build.mjs"], { stdio: "inherit" });
rmSync("stage", { recursive: true, force: true });
mkdirSync("stage", { recursive: true });
execFileSync("npx", ["web-ext", "build", "--source-dir", "dist", "--artifacts-dir", "stage", "--filename", "material-3-browser.xpi", "--overwrite-dest"], { stdio: "inherit" });
cpSync("dist", "stage/extension", { recursive: true });
