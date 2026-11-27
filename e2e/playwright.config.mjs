import { defineConfig } from "@playwright/test";
import { executable } from "../bench/navigateur.mjs";

export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.mjs",
  timeout: 180_000,
  workers: 1,
  expect: { timeout: 15_000 },
  use: {
    browserName: "chromium",
    headless: true,
    launchOptions: { executablePath: executable() },
    viewport: { width: 1440, height: 900 },
  },
});
