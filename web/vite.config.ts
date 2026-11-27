import solid from "@solidjs/vite-plugin";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [solid()],
  server: {
    host: "127.0.0.1",
    strictPort: true,
    proxy: {
      "/api": `http://${process.env.WEBMAIL_LISTEN ?? "127.0.0.1:8080"}`,
    },
  },
});
