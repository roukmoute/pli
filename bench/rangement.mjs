import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur } from "./navigateur.mjs";

const base = process.argv[2];
function banc(...args) {
  const script = fileURLToPath(new URL("./rangement.py", import.meta.url));
  return execFileSync("python3", ["-B", script, ...args], {
    encoding: "utf8",
  }).trim();
}
const [adresse, voisine] = banc("--preparer").split(" ");
// Lecture directe de la boîte, sans passer par le webmail.
const boite = () => JSON.parse(banc(adresse));
const api = (chemin) => new URL(`/api/v1/${chemin}`, base).href;

const navigateur = await ouvrirNavigateur();
try {
  // L’autre compte se connecte d’abord : le webmail connaît aussi ses dossiers.
  const autre = await navigateur.newContext();
  await connecter(await autre.newPage(), base, voisine, voisine);

  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 900 },
  });
  page.setDefaultTimeout(9000);
  await connecter(page, base, adresse, adresse);
  const ligne = (objet) => page.locator("main li", { hasText: objet });
  const coche = (objet) =>
    page.getByRole("checkbox", { name: `Sélectionner ${objet}`, exact: true });
  const commande = (nom) => page.getByRole("button", { name: nom, exact: true });
  const dialogue = page.getByRole("dialog");
  const depot = page.getByRole("complementary", {
    name: "Déposer dans un dossier",
  });
  const conclusion = () =>
    page.waitForFunction(
      () => !document.querySelector("main")?.textContent.includes("en attente"),
    );
  const operations = [];
  page.on("request", (requete) => {
    if (requete.method() === "POST" && requete.url().endsWith("/operations"))
      operations.push(requete.postDataJSON());
  });
  // Saisit la ligne et l’amène sur un dossier du sélecteur affiché pendant le glisser.
  async function glisser(objet, dossier, touche) {
    const depart = await ligne(objet).boundingBox();
    await page.mouse.move(depart.x + 200, depart.y + depart.height / 2);
    await page.mouse.down();
    await page.mouse.move(depart.x + 220, depart.y + 40, { steps: 4 });
    const cible = depot.getByText(dossier, { exact: true });
    await cible.waitFor();
    if (touche) await page.keyboard.down(touche);
    await cible.hover();
    await cible.hover();
    const annonce = await depot.getByRole("status").innerText();
    await page.mouse.up();
    if (touche) await page.keyboard.up(touche);
    return annonce;
  }
  assert.deepEqual(boite(), {
    reception: ["Fil A", "Fil B", "Fil C", "Fil D", "Fil E"],
    atelier: [],
    chantier: [],
  });

  // Par le menu : « Plus », Déplacer, puis le dossier dans le sélecteur.
  await coche("Fil B").check();
  await coche("Fil C").check();
  await commande("Plus").click();
  await commande("Déplacer").click();
  await dialogue.getByText("Déplacer 2 fils").waitFor();
  const proposes = await dialogue.getByRole("button").allInnerTexts();
  assert(proposes.includes("Atelier") && proposes.includes("Chantier"));
  const etrangers = proposes.filter((nom) => nom === "Voisinage").length;
  await dialogue.getByRole("button", { name: "Atelier", exact: true }).click();
  await ligne("Fil C").waitFor({ state: "detached" });
  await conclusion();
  assert.deepEqual(boite().atelier, ["Fil B", "Fil C"]);

  // Par le glisser : la ligne saisie emmène la sélection dont elle fait partie.
  await coche("Fil D").check();
  await coche("Fil E").check();
  assert.equal(await glisser("Fil D", "Atelier"), "Déplacer vers Atelier");
  await ligne("Fil E").waitFor({ state: "detached" });
  await conclusion();
  assert.deepEqual(boite(), {
    reception: ["Fil A"],
    atelier: ["Fil B", "Fil C", "Fil D", "Fil E"],
    chantier: [],
  });
  const [menu, glisse] = operations.map(({ type, vue, cibles, destination }) =>
    JSON.stringify({ type, vue, destination, lot: cibles.length }),
  );
  assert.equal(operations[0].type, "deplacer");
  const memes = Number(menu === glisse);

  // Ctrl pendant le glisser demande une copie : le fil reste aussi en Réception.
  if (process.env.PLI_CAPTURE_RANGEMENT) {
    const depart = await ligne("Fil A").boundingBox();
    await page.mouse.move(depart.x + 200, depart.y + depart.height / 2);
    await page.mouse.down();
    await page.mouse.move(depart.x + 220, depart.y + 40, { steps: 4 });
    await page.keyboard.down("Control");
    await depot.getByText("Chantier", { exact: true }).hover();
    await depot.getByText("Chantier", { exact: true }).hover();
    await page.screenshot({ path: process.env.PLI_CAPTURE_RANGEMENT });
    await page.keyboard.up("Control");
    await page.keyboard.press("Escape");
    await page.mouse.up();
    await depot.waitFor({ state: "detached" });
  }
  assert.equal(await glisser("Fil A", "Chantier", "Control"), "Copier vers Chantier");
  await page.getByText("Copie en attente").waitFor({ state: "detached" });
  await conclusion();
  assert.equal(operations.at(-1).type, "copier");
  const copiee = boite();
  const copies = Object.values(copiee).filter((objets) =>
    objets.includes("Fil A"),
  ).length;
  assert.deepEqual(copiee.chantier, ["Fil A"]);
  await ligne("Fil A").waitFor();

  // Le dossier choisi disparaît de la messagerie pendant que le sélecteur est ouvert.
  await coche("Fil A").check();
  await commande("Plus").click();
  await commande("Déplacer").click();
  await dialogue.getByRole("button", { name: "Chantier", exact: true }).waitFor();
  banc("--retirer", adresse, "Chantier");
  const session = await page.request.post(api("session"), {
    data: { address: adresse, password: adresse },
  });
  assert.equal(session.status(), 200);
  // La nouvelle découverte des dossiers retire celui qui n’existe plus.
  for (let essai = 0; ; essai++) {
    const dossiers = await (await page.request.get(api("folders"))).json();
    if (!dossiers.folders.some((d) => d.path === "Chantier")) break;
    assert(essai < 100, "Le dossier retiré est toujours annoncé.");
    await page.waitForTimeout(50);
  }
  await dialogue.getByRole("button", { name: "Chantier", exact: true }).click();
  await dialogue
    .getByRole("alert")
    .filter({
      hasText: "Ce dossier n’existe plus. Choisissez une autre destination.",
    })
    .waitFor();
  const annonces = await dialogue.getByText("Déplacer 1 fil").count();
  assert.equal(
    await dialogue.getByRole("button", { name: "Chantier", exact: true }).count(),
    0,
  );
  await ligne("Fil A").waitFor();
  assert.deepEqual(boite().reception, ["Fil A"]);
  await dialogue.getByRole("button", { name: "Atelier", exact: true }).click();
  await ligne("Fil A").waitFor({ state: "detached" });
  await conclusion();
  const rangee = boite();
  assert.deepEqual(rangee.reception, []);
  console.log(
    JSON.stringify({
      memes,
      copies,
      annonces,
      etrangers,
      ranges: rangee.atelier.length,
    }),
  );
} finally {
  await navigateur.close();
}
