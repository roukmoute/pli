import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur } from "./navigateur.mjs";

const base = process.argv[2];
function sonde(fichier, ...args) {
  return execFileSync(
    "python3",
    ["-B", fileURLToPath(new URL(fichier, import.meta.url)), ...args],
    { encoding: "utf8" },
  ).trim();
}
const adresse = sonde("./lecture.py", "0");
const nonLus = () => Number(sonde("./screener.py", "--unread", adresse));
const navigateur = await ouvrirNavigateur();
try {
  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 900 },
  });
  page.setDefaultTimeout(9000);
  await connecter(page, base, adresse, adresse);
  const ligne = (objet) => page.getByRole("button", { name: objet });
  const objetA = /Plan de montage/;
  const objetB = /Reçu de la librairie/;
  const objetC = /Été à 京都/;
  const a = ligne(objetA);
  const b = page.getByRole("checkbox", { name: objetB });
  const c = page.getByRole("checkbox", { name: objetC });
  const globale = page.getByRole("checkbox", { name: "Sélectionner la page" });
  // Le fil A commence à cheval sur les états lu et non lu.
  await a.focus();
  const confirmation = page.waitForResponse((r) => r.url().endsWith("/open"));
  await page.keyboard.press("Enter");
  assert.equal((await confirmation).status(), 200);
  await page.getByRole("button", { name: "Retour" }).click();
  const vus = page.locator('[data-section="deja_vus"]');
  await vus.getByRole("button", { name: objetA }).waitFor();
  await a.focus();
  const ancre = await a.getAttribute("data-fil");
  const avant = nonLus();
  let ouvertures = 0;
  page.on("request", (r) => {
    if (r.url().endsWith("/open")) ouvertures++;
  });
  await b.check();
  await c.focus();
  await page.keyboard.press("Space");
  assert(await c.isChecked());
  assert(await globale.evaluate((el) => el.indeterminate));
  const annonce = page
    .getByRole("status")
    .filter({ hasText: /fils sélectionnés/ });
  const nombre = Number(
    (await annonce.innerText()).match(
      /^(\d+) fils sélectionnés sur \d+$/u,
    )?.[1],
  );
  if (process.env.PLI_CAPTURE_SELECTION)
    await page.screenshot({ path: process.env.PLI_CAPTURE_SELECTION });
  const etoile = page
    .getByRole("listitem")
    .filter({ has: b })
    .getByRole("button", { name: "Étoile", exact: true });
  const soumission = () =>
    page.waitForResponse(
      (r) =>
        r.url().endsWith("/messages/flags") && r.request().method() === "POST",
    );
  const attente = soumission();
  await etoile.click();
  assert.equal((await attente).status(), 200);
  await page.waitForFunction(() =>
    document.querySelector('button[aria-label="Étoile"][aria-pressed="true"]'),
  );
  assert.equal(await etoile.getAttribute("aria-pressed"), "true");
  assert(await b.isChecked());
  assert.equal(new URL(page.url()).pathname, "/reception");
  await page
    .getByRole("button", { name: "Marquer comme lu", exact: true })
    .click();
  await vus.getByRole("button", { name: objetB }).waitFor();
  await vus.getByRole("button", { name: objetC }).waitFor();
  assert.equal(
    await vus.getByRole("button", { name: objetA }).getAttribute("data-fil"),
    ancre,
  );
  const marques = avant - nonLus();
  // La navigation revient sur le repère de lecture A après l'action sur B et C.
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("link", { name: "Flux", exact: true }).click();
  await page.getByRole("heading", { name: "Flux", exact: true }).waitFor();
  await page.goBack();
  await page.waitForFunction(
    (id) => document.activeElement?.dataset.fil === id,
    ancre,
  );
  await b.check();
  await c.check();
  await page
    .getByRole("button", { name: "Marquer comme non lu", exact: true })
    .click();
  const nouveaux = page.locator('[data-section="nouveaux"]');
  await nouveaux.getByRole("button", { name: objetB }).waitFor();
  await nouveaux.getByRole("button", { name: objetC }).waitFor();
  assert.equal(nonLus(), avant);
  assert(await vus.getByRole("button", { name: objetA }).isVisible());

  const autre = await navigateur.newPage();
  await connecter(autre, base, "bob@recette.test", "bob-test");
  const filB = await ligne(objetB).getAttribute("data-fil");
  const messages = await page.request.get(
    new URL("/api/v1/threads/" + encodeURIComponent(filB), base).href,
  );
  const [lettre] = await messages.json();
  const reponse = await autre.request.post(
    new URL("/api/v1/messages/flags", base).href,
    {
      data: { ids: [lettre.id], indicateur: "lu", valeur: true },
    },
  );
  assert.equal(reponse.status(), 200);
  const [resultat] = await reponse.json();
  assert.equal(resultat.id, lettre.id);
  assert.equal(nonLus(), avant);
  console.log(
    JSON.stringify({
      selection: nombre,
      marques,
      ouvertures,
      refus: Number(resultat.etat === "absent"),
    }),
  );
} finally {
  await navigateur.close();
}
