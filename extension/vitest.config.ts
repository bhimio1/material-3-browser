import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    // The package uses extension-less ESM imports; let Vite resolve them.
    server: { deps: { inline: ["@material/material-color-utilities"] } },
  },
});
