// Writes a sample end-4 style colors.json (one dark scheme) and the full
// palette message for rendering pages outside the browser.
import { writeFileSync } from "node:fs";
import { seedPalette } from "../../src/shared/palette";

const seed = process.argv[2] ?? "#7A5AB8";
const p = seedPalette(seed, true);
writeFileSync(process.argv[3] ?? "out/colors.json", JSON.stringify(p.dark, null, 2));
writeFileSync(process.argv[4] ?? "out/palette.json", JSON.stringify(p, null, 2));
