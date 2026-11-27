import { expect } from "@playwright/test";
import { connecter } from "../bench/navigateur.mjs";
import { alice, livrer, secret, test } from "./banc.mjs";
import { observerRequetes, retour } from "../bench/observer-retour.mjs";
import { mesurerRetours } from "./mesure.mjs";

const correspondant = "reliure@recette.test";
const prospection = "prospection@recette.test";
const sujet = "Re: Les épreuves sont prêtes";
const tri = (page) => page.getByRole("button", { name: /Nouveaux expéditeurs/ });
const expediteur = (page, adresse) => page.getByRole("article", { name: adresse, exact: true });
const fil = (page) => page.locator("[data-fil]").filter({ hasText: sujet });

async function lireApi(page, chemin) {
  const reponse = await page.request.get(new URL("/api/v1/" + chemin, page.url()).href);
  expect(reponse.ok()).toBeTruthy();
  return reponse.json();
}

async function decider(page, adresse, choix) {
  const reponse = page.waitForResponse((r) =>
    r.url().endsWith("/api/v1/screener/decisions") && r.request().method() === "POST",
  );
  await expediteur(page, adresse).getByRole("button", { name: choix, exact: true }).click();
  expect((await reponse).status()).toBe(200);
}

async function ouvrirFil(page) {
  await fil(page).evaluate((el) => el.click());
  await expect(page.getByRole("heading", { name: sujet, exact: true })).toBeVisible();
  await expect(page.frameLocator("iframe").getByText(/Les épreuves sont prêtes/)).toBeVisible();
}

test("Alice retrouve son courrier et ses décisions sur ordinateur et téléphone", async ({ page, browser, banc }) => {
  await observerRequetes(page);
  await connecter(page, banc.base, alice, secret);
  await banc.lotSuspendu();
  await expect(page.locator("[data-fil]").filter({ hasText: "Bulletin de la bibliothèque" }).first()).toBeVisible();
  expect((await lireApi(page, "views/reception")).couverture.import_en_cours).toBe(true);
  await livrer("correspondant");
  await expect(tri(page).locator("strong")).toHaveText("1");
  await expect(fil(page)).toHaveCount(0);
  const avant = await lireApi(page, "views/reception");
  expect([...avant.nouveaux, ...avant.deja_vus].some((f) => f.expediteur.adresse === correspondant)).toBe(false);
  await tri(page).click();
  await expect(page.getByRole("article")).toHaveCount(1);
  const demande = expediteur(page, correspondant);
  expect((await lireApi(page, "screener")).senders).toEqual([
    expect.objectContaining({ address: correspondant, message_count: 2 }),
  ]);
  await expect(demande).toContainText(sujet);

  await banc.redemarrer();
  await page.reload();
  await expect(demande).toBeVisible();
  await demande.getByLabel("Accepter vers").selectOption("reception");
  await decider(page, correspondant, "Accepter");
  await page.getByRole("link", { name: "Réception", exact: true }).click();
  await ouvrirFil(page);
  await expect(page.locator("main article")).toHaveCount(2);
  await page.getByRole("button", { name: /Retour/ }).click();
  await expect(fil(page)).toBeVisible();

  await page.setViewportSize({ width: 390, height: 844 });
  await ouvrirFil(page);
  const mobile = await retour(page.getByRole("button", { name: /Retour/ }));
  expect(mobile.requetes_avant_rendu).toBe(0);
  await expect(fil(page)).toBeInViewport();
  await expect(fil(page)).toBeFocused();

  await livrer("prospection");
  await expect.poll(async () =>
    (await lireApi(page, "screener")).senders.map((s) => s.address),
  ).toContain(prospection);
  await tri(page).click();
  await decider(page, prospection, "Refuser");
  await banc.redemarrer();
  await page.reload();
  await page.locator("header summary").click();
  await page.getByRole("link", { name: "Historique du tri" }).click();
  await expect(expediteur(page, correspondant)).toContainText("Réception");
  await expect(expediteur(page, prospection)).toContainText("Refusé");
  await page.getByRole("link", { name: "Réception", exact: true }).click();
  await expect(fil(page)).toBeVisible();
  await expect(page.getByText("Offre de démonstration", { exact: true })).toHaveCount(0);

  await page.setViewportSize({ width: 1440, height: 900 });
  await mesurerRetours(page, browser, () => ouvrirFil(page));
});
