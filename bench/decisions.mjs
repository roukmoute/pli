import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { connecter, ouvrirNavigateur, tailles } from "./navigateur.mjs";

const base = new URL(process.argv[2]);
const adresse = process.argv[3];
const li = { name: "atelier@recette.test", exact: true };
const sujets = ["Archive A", "Archive B", "Arrivée A", "Arrivée B"];

async function relever(page, etat) {
  const dossier = process.env.PLI_CAPTURES_TRI;
  for (const taille of tailles) {
    await page.setViewportSize(taille);
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    if (taille.width > 640) {
      const menu = await page.getByRole("button", { name: "Menu", exact: true }).boundingBox();
      const compte = await page.locator("header summary").boundingBox();
      assert(menu.x + menu.width <= compte.x, "L’adresse empiète sur le menu.");
    }
    if (dossier) {
      mkdirSync(dossier, { recursive: true });
      await page.screenshot({ path: join(dossier, `${taille.width}-${etat}.png`) });
    }
  }
  return tailles.length;
}

async function menu(page, vue) {
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("dialog").getByRole("link", { name: vue, exact: true }).click();
  await page.getByRole("heading", { name: vue, exact: true }).waitFor();
}

async function verifierCourrier(page) {
  for (const sujet of sujets) await page.getByText(sujet, { exact: true }).waitFor();
  return page.locator("[data-fil]").count();
}

const post = (r) => r.url().endsWith("/api/v1/screener/decisions") && r.request().method() === "POST";

async function accepterSansAttendre(page) {
  const depart = Promise.withResolvers();
  const continuer = Promise.withResolvers();
  const chemin = "**/api/v1/screener/decisions";
  await page.route(chemin, async (route) => {
    depart.resolve();
    await continuer.promise;
    await route.continue();
  });
  const reponse = page.waitForResponse(post);
  try {
    const ligne = page.getByRole("article", li);
    await ligne.getByLabel("Accepter vers").selectOption("flux");
    await ligne.getByRole("button", { name: "Accepter", exact: true }).click();
    await depart.promise;
    await page.getByText("Aucun nouvel expéditeur à trier.").waitFor();
    await page.getByRole("link", { name: "Réception", exact: true }).click();
    const compteur = page.getByRole("button", { name: /Nouveaux expéditeurs/ }).locator("strong");
    assert.equal(await compteur.innerText(), "0");
    assert.equal(await page.locator("[data-fil]").count(), 0);
    await menu(page, "Flux");
    return await verifierCourrier(page);
  } finally {
    continuer.resolve();
    assert.equal((await reponse).status(), 200);
    await page.unroute(chemin);
  }
}

async function changerDepuisLeSecondOnglet(page) {
  const decision = page.waitForResponse(post);
  await page.getByRole("article", li).getByRole("button", { name: "Refuser" }).click();
  const statut = (await decision).status();
  const avis = page.getByRole("alert");
  await avis.getByText(/Une autre décision a été enregistrée/).waitFor();
  assert.match(await avis.innerText(), /atelier@recette\.test · Flux/);
  const modification = page.waitForResponse(post);
  await avis.getByRole("button", { name: "Refuser" }).click();
  assert.equal((await modification).status(), 200);
  await avis.waitFor({ state: "hidden" });
  return statut;
}

async function restaurer(page) {
  await menu(page, "Flux");
  await page.getByText("Aucun nouveau fil.", { exact: true }).waitFor();
  assert.equal(await page.locator("[data-fil]").count(), 0);
  await page.locator("summary").filter({ hasText: adresse }).click();
  await page.getByRole("link", { name: "Historique du tri" }).click();
  const ligne = page.getByRole("article", li);
  await ligne.getByText("Refusé", { exact: true }).waitFor();
  await ligne.locator("summary").click();
  for (const sujet of sujets) await ligne.locator("details").getByText(sujet, { exact: true }).waitFor();
  const historique = await ligne.locator("details small").count();
  await relever(page, "historique");
  await ligne.getByLabel("Accepter vers").focus();
  await page.keyboard.press("End");
  assert.equal(await ligne.getByLabel("Accepter vers").inputValue(), "justificatifs");
  const reponse = page.waitForResponse(post);
  await ligne.getByRole("button", { name: "Accepter", exact: true }).click();
  assert.equal((await reponse).status(), 200);
  await menu(page, "Justificatifs");
  await page.reload();
  return { historique, justificatifs: await verifierCourrier(page) };
}

const navigateur = await ouvrirNavigateur();
try {
  const contexte = await navigateur.newContext();
  const page = await contexte.newPage();
  contexte.setDefaultTimeout(7000);
  await connecter(page, base, adresse, adresse);
  await page.getByText("Archive A", { exact: true }).waitFor();
  await page.getByRole("button", { name: /Nouveaux expéditeurs/ }).click();
  const ligne = page.getByRole("article", li);
  await ligne.getByRole("heading", { name: "L’Atelier" }).waitFor();
  await ligne.getByText("Arrivée B", { exact: true }).waitFor();
  await ligne.getByText("Lettre de l’atelier. Arrivée B.", { exact: true }).waitFor();
  assert.equal(await page.getByRole("article").count(), 1);
  const formats = await relever(page, "screener");
  const autre = await contexte.newPage();
  await autre.goto(new URL("/nouveaux-expediteurs", base).href);
  await autre.getByRole("article", li).waitFor();
  const flux = await accepterSansAttendre(page);
  const conflit = await changerDepuisLeSecondOnglet(autre);
  const suite = await restaurer(autre);
  console.log(JSON.stringify({ formats, flux, conflit, ...suite }));
} finally {
  await navigateur.close();
}
