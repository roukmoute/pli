import solid from "@solidjs/vite-plugin";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    unstubGlobals: true,
    coverage: {
      include: ["src/**/*.{ts,tsx,mts,cts}"],
      exclude: ["src/**/*.test.*", "src/test-setup.ts"],
      reporter: ["lcovonly"],
      reportsDirectory: "../target/coverage/typescript",
    },
  },
  server: {
    // Les fichiers inchangés des workers de mutator sont des liens vers web/.
    fs: process.env.PLI_MUTATION_WEB
      ? { allow: [".", process.env.PLI_MUTATION_WEB] }
      : undefined,
    host: "127.0.0.1",
    strictPort: true,
    proxy: {
      "/api": `http://${process.env.WEBMAIL_LISTEN ?? "127.0.0.1:8080"}`,
    },
  },
});
