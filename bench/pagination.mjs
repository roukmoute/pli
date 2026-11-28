import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur } from "./navigateur.mjs";

const base = process.argv[2];
const expediteur = "atelier@recette.test";
function banc(...args) {
  const script = fileURLToPath(new URL("./pagination.py", import.meta.url));
  return execFileSync("python3", ["-B", script, ...args], {
    encoding: "utf8",
  }).trim();
}

async function api(page, chemin, options) {
  const reponse = await page.request.fetch(
    new URL("/api/v1" + chemin, base).href,
    options,
  );
  assert(reponse.ok(), `${chemin} a répondu ${reponse.status()}.`);
  return reponse.json();
}

// Interroge le service jusqu’à ce que la réponse satisfasse la condition.
async function attendre(page, chemin, condition) {
  for (let essai = 0; essai < 240; essai++) {
    const reponse = await api(page, chemin);
    if (condition(reponse)) return reponse;
    await page.waitForTimeout(50);
  }
  throw new Error(`${chemin} n’a pas atteint l’état attendu.`);
}

const fils = (reponse) => [...reponse.nouveaux, ...reponse.deja_vus];
const lignes = (page) => page.locator("main [data-fil]");
const presents = (page) =>
  lignes(page).evaluateAll((els) => els.map((el) => el.dataset.fil));

async function menu(page, destination) {
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("link", { name: destination, exact: true })
    .click();
  await page
    .getByRole("heading", { name: destination, exact: true, level: 1 })
    .waitFor();
}

const nombre = (page) => lignes(page).count();

// Les premières pages s’enchaînent seules tant que la zone n’est pas remplie.
async function attendreLeRepos(page, suivants) {
  let stable = 0;
  for (let avant = -1; stable < 3; ) {
    await page.waitForTimeout(100);
    const present = await nombre(page);
    const occupe = (await suivants.getAttribute("aria-busy")) === "true";
    stable = present === avant && !occupe ? stable + 1 : 0;
    avant = present;
  }
}

async function chargerToutesLesPages(page, total) {
  const suivants = page.getByRole("button", {
    name: "Afficher les fils suivants",
    exact: true,
  });
  await suivants.waitFor();
  await attendreLeRepos(page, suivants);
  // Le repli se commande au clavier.
  const avant = await nombre(page);
  assert(avant < total, "Le repli doit rester à l’écran après le premier lot.");
  await suivants.focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction(
    (lignes) => document.querySelectorAll("main [data-fil]").length > lignes,
    avant,
  );
  // Chaque passage défile jusqu’en bas : l’approche demande la page suivante.
  for (let passage = 0; passage < 80; passage++) {
    if ((await nombre(page)) === total) return;
    await page.locator("main").evaluate((zone) => {
      zone.scrollTop = zone.scrollHeight;
    });
    await page.waitForTimeout(100);
  }
  throw new Error("La liste n’a pas chargé toutes ses pages.");
}

const adresse = banc();
const navigateur = await ouvrirNavigateur();
try {
  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 720 },
  });
  page.setDefaultTimeout(9000);
  await connecter(page, base, adresse, adresse);
  // La lettre attendra au tri pendant que la lecture avance dans les pages.
  banc("--arrivee", adresse);
  const complete = await attendre(
    page,
    "/views/reception?taille=100",
    (liste) => !liste.couverture.import_en_cours && fils(liste).length === 20,
  );
  const taille = fils(await api(page, "/views/reception")).length;
  assert(taille < 20, "La première page doit laisser une suite à charger.");

  await chargerToutesLesPages(page, 20);
  const charges = await presents(page);
  assert.deepEqual(
    charges,
    fils(complete).map((fil) => fil.id),
    "Les pages réunies doivent reproduire la liste du service.",
  );
  assert.equal(new Set(charges).size, charges.length);
  assert.equal(
    await page.getByRole("button", { name: /Afficher les fils/ }).count(),
    0,
  );

  // Dixième ligne : avec quatre fils par page, elle appartient à la troisième.
  const rang = 9;
  const ligne = await lignes(page).nth(rang).elementHandle();
  await ligne.scrollIntoViewIfNeeded();
  await ligne.focus();
  const cible = charges[rang];
  const defilement = await page.locator("main").evaluate(async (zone) => {
    await new Promise((suite) =>
      requestAnimationFrame(() => requestAnimationFrame(suite)),
    );
    return zone.scrollTop;
  });
  assert(defilement > 0, "La troisième page se lit après un défilement.");

  await menu(page, "Atelier");
  await page.getByText("Aucun nouveau fil.", { exact: true }).waitFor();
  const { expediteurs } = await attendre(
    page,
    "/screener?details=true",
    (tri) => tri.expediteurs.some((e) => e.adresse === expediteur),
  );
  const attente = expediteurs.find((e) => e.adresse === expediteur);
  await api(page, "/screener/decisions", {
    method: "POST",
    data: {
      adresse: expediteur,
      decision: "accepte",
      destination: "reception",
      revision_attendue: attente.revision,
    },
  });
  await attendre(
    page,
    "/views/reception",
    (liste) => fils(liste)[0].objet === "Arrivée du soir",
  );

  // La relecture reste retenue le temps de poser le pointeur sur la ligne.
  const relecture = Promise.withResolvers();
  const retenir = async (route) => {
    await relecture.promise;
    await route.continue();
  };
  const reception = (url) => url.pathname === "/api/v1/views/reception";
  await page.route(reception, retenir);
  let ecart;
  let annonce;
  try {
    await menu(page, "Réception");
    await page.waitForFunction(
      (id) => document.activeElement?.getAttribute("data-fil") === id,
      cible,
    );
    assert.deepEqual(await presents(page), charges);
    const retour = await page.locator("main").evaluate((zone) => ({
      defilement: zone.scrollTop,
      rang: Array.from(zone.querySelectorAll("[data-fil]")).indexOf(
        document.activeElement,
      ),
    }));
    assert(Math.abs(retour.defilement - defilement) <= 1);
    assert.equal(retour.rang, rang);
    // La liste a été redessinée : la ligne au focus est un nouvel élément.
    const retrouvee = await lignes(page).nth(rang).elementHandle();
    await retrouvee.hover();
    const avant = await retrouvee.boundingBox();
    relecture.resolve();
    const acces = page.getByRole("button", { name: /Nouveautés/ });
    await acces.waitFor();
    const apres = await retrouvee.boundingBox();
    ecart = Math.abs(apres.y - avant.y);
    annonce = Number(await acces.locator("strong").innerText());
    assert.equal(await retrouvee.getAttribute("data-fil"), cible);
    assert(await retrouvee.evaluate((el) => el === document.activeElement));
    assert((await presents(page)).length === charges.length + annonce);
    if (process.env.PLI_CAPTURE_PAGINATION)
      await page.screenshot({ path: process.env.PLI_CAPTURE_PAGINATION });
    await acces.click();
    await acces.waitFor({ state: "detached" });
    const haut = await page.locator("main").evaluate((zone) => ({
      defilement: zone.scrollTop,
      objet: document.activeElement?.textContent,
    }));
    assert.equal(haut.defilement, 0);
    assert.match(haut.objet, /Arrivée du soir/);
  } finally {
    relecture.resolve();
    // Les relectures déjà parties terminent avant le retrait de la retenue.
    await page.unrouteAll({ behavior: "wait" });
  }
  console.log(
    JSON.stringify({
      fils: charges.length,
      page: Math.floor(rang / taille) + 1,
      // Un écart inférieur au pixel tient à l’arrondi du défilement.
      ecart: Math.round(ecart < 1 ? 0 : ecart),
      arrivees: annonce,
    }),
  );
} finally {
  await navigateur.close();
}
