import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
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
const boite = (commande, objet) =>
  JSON.parse(sonde("./operations.py", commande, adresse, objet));
const api = (chemin) => new URL(`/api/v1/${chemin}`, base).href;

const navigateur = await ouvrirNavigateur();
try {
  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 900 },
  });
  page.setDefaultTimeout(9000);
  await connecter(page, base, adresse, adresse);
  const jardin = "Le jardin en images";
  const ligne = (objet) => page.getByRole("button", { name: objet });
  const rangee = (objet) =>
    page.getByRole("listitem").filter({ has: ligne(objet) });
  const etoile = (objet) =>
    rangee(objet).getByRole("button", { name: "Étoile", exact: true });
  const section = (nom) =>
    page
      .locator(`[data-section="${nom}"]`)
      .getByRole("button", { name: jardin });
  const marquer = async (commande) => {
    await rangee(jardin).getByRole("checkbox").check();
    await page.getByRole("button", { name: commande, exact: true }).click();
  };
  assert.deepEqual(boite("etat", jardin), { non_lus: 1, etoiles: 0 });

  // Les trois envois restent retenus : la liste ne peut refléter que le geste local.
  const retenus = [];
  await page.route("**/api/v1/operations", async (route) => {
    const feu = Promise.withResolvers();
    retenus.push({ requete: route.request(), feu });
    await feu.promise;
    await route.continue();
  });
  const sansReponse = { timeout: 1000 };
  let immediats = 0;
  await etoile(jardin).click();
  await rangee(jardin)
    .locator('[aria-pressed="true"][data-attente="oui"]')
    .waitFor(sansReponse);
  immediats++;
  await marquer("Marquer comme lu");
  await section("deja_vus").waitFor(sansReponse);
  await section("deja_vus").getByText("En attente").waitFor(sansReponse);
  immediats++;
  if (process.env.PLI_CAPTURE_OPERATIONS) {
    // Les signes d’attente se montrent après un court délai.
    await page.waitForTimeout(800);
    await page.screenshot({ path: process.env.PLI_CAPTURE_OPERATIONS });
  }
  await marquer("Marquer comme non lu");
  await section("nouveaux").waitFor(sansReponse);
  immediats++;
  const demandes = retenus.map(({ requete }) => requete.postDataJSON());
  assert.deepEqual(
    demandes.map((demande) => [demande.type, demande.vue]),
    [
      ["etoile", "reception"],
      ["lu", "reception"],
      ["non_lu", "reception"],
    ],
  );
  assert.deepEqual(boite("etat", jardin), { non_lus: 1, etoiles: 0 });

  // Les envois repartent dans leur ordre, puis la messagerie confirme chacun.
  for (const { requete, feu } of retenus) {
    feu.resolve();
    assert.equal((await requete.response()).status(), 200);
  }
  await page.unrouteAll({ behavior: "wait" });
  await page.waitForFunction(
    () =>
      !document.querySelector("[data-attente]") &&
      !document.querySelector("main")?.textContent.includes("En attente"),
  );
  await section("nouveaux").waitFor();
  assert.equal(await etoile(jardin).getAttribute("aria-pressed"), "true");
  const { non_lus, etoiles } = boite("etat", jardin);
  assert.equal(etoiles, 1);

  // Rejouer l’opération « lu », déjà suivie de « non lu », ne réapplique rien.
  const rejeu = await page.request.post(api("operations"), {
    data: demandes[1],
  });
  const connu = await page.request.get(api(`operations/${demandes[1].id}`));
  assert.equal(rejeu.status(), 200);
  const resultat = await rejeu.json();
  assert.equal(resultat.etat, "confirmee");
  const identiques = Number(
    JSON.stringify(resultat) === JSON.stringify(await connu.json()),
  );
  assert.deepEqual(boite("etat", jardin), { non_lus, etoiles });

  // Une autre boîte ne peut pas atteindre cette lettre.
  const fil = await ligne(jardin).getAttribute("data-fil");
  const messages = await page.request.get(
    api(`threads/${encodeURIComponent(fil)}`),
  );
  const [lettre] = await messages.json();
  const autre = await navigateur.newPage();
  await connecter(autre, base, "bob@recette.test", "bob-test");
  const etrangere = await autre.request.post(api("operations"), {
    data: { id: randomUUID(), type: "lu", cibles: [lettre.id] },
  });
  assert.equal(etrangere.status(), 200);
  const refus = await etrangere.json();
  const refusees = Number(
    refus.etat === "refusee" && refus.resultats[0].etat === "absent",
  );
  assert.deepEqual(boite("etat", jardin), { non_lus, etoiles });

  // La lettre de bienvenue disparaît de la boîte : la messagerie ne confirme pas son étoile.
  const bienvenue = "Bienvenue au jardin";
  boite("retirer", bienvenue);
  await etoile(bienvenue).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Modification non confirmée pour 1 fil." })
    .waitFor();
  await rangee(bienvenue).locator('[aria-pressed="false"]').waitFor();
  const retablis = await rangee(bienvenue)
    .locator('[aria-pressed="false"]:not([data-attente])')
    .count();
  console.log(
    JSON.stringify({ immediats, non_lus, identiques, refusees, retablis }),
  );
} finally {
  await navigateur.close();
}
