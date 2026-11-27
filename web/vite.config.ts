import solid from "@solidjs/vite-plugin";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    unstubGlobals: true,
  },
  server: {
    host: "127.0.0.1",
    strictPort: true,
    proxy: {
      "/api": `http://${process.env.WEBMAIL_LISTEN ?? "127.0.0.1:8080"}`,
    },
  },
});
