import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const requireWeb = createRequire(
  new URL("../web/package.json", import.meta.url),
);
const { chromium } = requireWeb("playwright-core");
export const tailles = [
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1440, height: 900 },
];

export function executable() {
  const personnalise = process.env.PLI_CHROMIUM_BIN;
  if (personnalise) return personnalise;
  return [
    chromium.executablePath(),
    "/usr/bin/chromium",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium-browser",
  ].find(existsSync);
}

export function preparer() {
  if (executable()) return;
  const cli = join(
    dirname(requireWeb.resolve("playwright-core/package.json")),
    "cli.js",
  );
  execFileSync(process.execPath, [cli, "install", "chromium"], {
    stdio: "inherit",
  });
}

export async function connecter(page, base, adresse, secret) {
  await page.goto(new URL("/reception", base).href);
  await page.getByLabel("Adresse de messagerie").fill(adresse);
  await page.getByLabel("Secret de messagerie").fill(secret);
  await page.getByRole("button", { name: "Se connecter" }).click();
  await page.getByRole("heading", { name: "Réception", exact: true }).waitFor();
}

export function ouvrirNavigateur() {
  return chromium.launch({ executablePath: executable(), headless: true });
}
