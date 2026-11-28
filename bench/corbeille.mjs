import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur } from "./navigateur.mjs";

const base = process.argv[2];
function banc(...args) {
  const script = fileURLToPath(new URL("./corbeille.py", import.meta.url));
  return execFileSync("python3", ["-B", script, ...args], {
    encoding: "utf8",
  }).trim();
}
const adresse = banc("--preparer");
// Lecture directe de la boîte, sans passer par le webmail.
const boite = () => JSON.parse(banc(adresse));
const api = (chemin) => new URL(`/api/v1/${chemin}`, base).href;

const navigateur = await ouvrirNavigateur();
try {
  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 900 },
  });
  page.setDefaultTimeout(9000);
  await connecter(page, base, adresse, adresse);
  const ligne = (objet) => page.locator("main [data-fil]", { hasText: objet });
  const coche = (objet) =>
    page.getByRole("checkbox", { name: `Sélectionner ${objet}`, exact: true });
  const commande = (nom) => page.getByRole("button", { name: nom, exact: true });
  const dialogue = page.getByRole("dialog");
  // La messagerie a confirmé quand plus aucun signe d’attente ni refus ne s’affiche.
  const conclusion = () =>
    page.waitForFunction(
      () =>
        !document.querySelector("[role=alert]") &&
        !document.querySelector("main")?.textContent.includes("en attente"),
    );
  async function ouvrir(destination) {
    await page.getByRole("button", { name: "Menu", exact: true }).click();
    await dialogue.getByRole("link", { name: destination, exact: true }).click();
    await page
      .getByRole("heading", { name: destination, exact: true, level: 1 })
      .waitFor();
  }
  async function total(liste) {
    const reponse = await page.request.get(api(liste));
    assert.equal(reponse.status(), 200);
    return (await reponse.json()).total;
  }
  assert.deepEqual(boite(), {
    reception: ["Fil A", "Fil B", "Fil C", "Fil D"],
    corbeille: [],
  });

  // Supprimer part sans confirmation, réponse retenue : la ligne a déjà quitté la liste.
  await coche("Fil B").check();
  await commande("Supprimer la sélection").waitFor();
  const retenus = [];
  await page.route("**/api/v1/operations", async (route) => {
    const feu = Promise.withResolvers();
    retenus.push({ requete: route.request(), feu });
    await feu.promise;
    await route.continue();
  });
  const avant = await page.locator("main [data-fil]").count();
  await commande("Supprimer la sélection").click();
  await ligne("Fil B").waitFor({ state: "detached", timeout: 1000 });
  const immediats = avant - (await page.locator("main [data-fil]").count());
  assert.equal(await dialogue.count(), 0);
  assert.equal(retenus.length, 1);
  assert.equal(retenus[0].requete.postDataJSON().type, "corbeille");
  assert.deepEqual(boite().corbeille, []);
  retenus[0].feu.resolve();
  assert.equal((await retenus[0].requete.response()).status(), 200);
  await page.unrouteAll({ behavior: "wait" });
  await conclusion();
  assert.deepEqual(boite(), {
    reception: ["Fil A", "Fil C", "Fil D"],
    corbeille: ["Fil B"],
  });

  // Dans la Corbeille, Restaurer ramène le fil dans son dossier d’origine.
  const dossiers = await (await page.request.get(api("folders"))).json();
  const corbeille = dossiers.folders.find((d) => d.role === "trash");
  const jetes = `folders/${corbeille.id}/messages`;
  assert.deepEqual(
    [await total("views/reception"), await total(jetes)],
    [3, 1],
  );
  await ouvrir("Corbeille");
  await coche("Fil B").check();
  await commande("Restaurer la sélection").click();
  await ligne("Fil B").waitFor({ state: "detached" });
  await conclusion();
  const rendue = boite();
  assert.deepEqual(rendue.corbeille, []);
  const revenus = rendue.reception.filter((objet) => objet === "Fil B").length;
  const reception = await total("views/reception");
  const restants = await total(jetes);
  await ouvrir("Réception");
  await ligne("Fil B").waitFor();

  // Deux autres fils partent pour la Corbeille.
  await coche("Fil C").check();
  await coche("Fil D").check();
  await commande("Supprimer la sélection").click();
  await ligne("Fil D").waitFor({ state: "detached" });
  await conclusion();
  assert.deepEqual(boite().corbeille, ["Fil C", "Fil D"]);
  await ouvrir("Corbeille");
  await ligne("Fil C").waitFor();

  // Le vidage cite le dossier et le nombre de fils. Annulé, il ne supprime rien.
  await commande("Vider le dossier").click();
  await dialogue
    .getByText("2 fils seront supprimés définitivement de Corbeille.")
    .waitFor();
  if (process.env.PLI_CAPTURE_CORBEILLE)
    await page.screenshot({ path: process.env.PLI_CAPTURE_CORBEILLE });
  await page.keyboard.press("Escape");
  await dialogue.waitFor({ state: "detached" });
  const gardes = boite().corbeille.length;
  assert.equal(await page.locator("main [data-fil]").count(), 2);

  // La suppression définitive n’efface que le fil ciblé.
  await coche("Fil C").check();
  await commande("Supprimer définitivement").click();
  await dialogue
    .getByText("1 fil sera supprimé définitivement de Corbeille.")
    .waitFor();
  await dialogue
    .getByRole("button", { name: "Supprimer définitivement", exact: true })
    .click();
  await ligne("Fil C").waitFor({ state: "detached" });
  await conclusion();
  const epargnes = boite();
  assert.deepEqual(epargnes.corbeille, ["Fil D"]);
  assert.deepEqual(epargnes.reception, ["Fil A", "Fil B"]);

  // Confirmé, le vidage emporte ce qui reste.
  await commande("Vider le dossier").click();
  await dialogue
    .getByRole("button", { name: "Vider le dossier", exact: true })
    .click();
  await ligne("Fil D").waitFor({ state: "detached" });
  await conclusion();
  const videe = boite();
  assert.deepEqual(videe.reception, ["Fil A", "Fil B"]);
  console.log(
    JSON.stringify({
      immediats,
      revenus,
      reception,
      restants,
      gardes,
      epargnes: epargnes.corbeille.length,
      videe: videe.corbeille.length,
    }),
  );
} finally {
  await navigateur.close();
}
