import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { connecter, ouvrirNavigateur, preparer, tailles } from "./navigateur.mjs";

const captures = process.env.PLI_CAPTURES_NAVIGATION;

async function capture(page, etat, taille) {
  if (!captures) return;
  mkdirSync(captures, { recursive: true });
  await page.screenshot({
    path: join(captures, `${taille.width}-${etat}.png`),
  });
}

async function verifier(page, taille) {
  await page.setViewportSize(taille);
  const fil = page.locator("[data-fil]").nth(2);
  await fil.waitFor();
  await fil.focus();
  if (captures)
    await page.locator("main").evaluate((main) => {
      main.scrollTop = 0;
    });
  await capture(page, "reception", taille);
  await page.keyboard.press("h");
  const menu = page.getByRole("dialog", { name: "Menu" });
  await menu.getByRole("link", { name: "Atelier", exact: true }).waitFor();
  assert(
    await menu
      .getByRole("searchbox")
      .evaluate((el) => el === document.activeElement),
  );
  await page.keyboard.press("Tab");
  assert(await menu.evaluate((el) => el.contains(document.activeElement)));
  await capture(page, "menu", taille);
  await page.keyboard.press("Escape");
  const declencheur = page.getByRole("button", { name: "Menu", exact: true });
  assert(await declencheur.evaluate((el) => el === document.activeElement));
  await fil.focus();
  const avant = await fil.evaluate(async (el) => {
    const zone = el.closest("main");
    zone.scrollTop = 164;
    await new Promise((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(resolve)),
    );
    return {
      id: el.dataset.fil,
      section: el.closest("section").dataset.section,
      scroll: zone.scrollTop,
    };
  });
  assert(avant.scroll > 0, "La liste doit avoir défilé avant le départ.");
  for (const commande of ["menu", "historique"]) {
    await verifierRetour(page, avant, commande);
  }
}

async function verifierRetour(page, avant, commande) {
  const declencheur = page.getByRole("button", { name: "Menu", exact: true });
  const menu = page.getByRole("dialog", { name: "Menu" });
  await declencheur.click();
  await menu.getByRole("link", { name: "Atelier", exact: true }).click();
  await page.getByRole("heading", { name: "Atelier", exact: true }).waitFor();
  await page.getByText("Aucun nouveau fil.", { exact: true }).waitFor();
  // Seule l'actualisation suivante est suspendue. La première page est celle du service.
  let reprendre;
  const barrage = new Promise((r) => {
    reprendre = r;
  });
  let requetes = 0;
  const arrivee = Promise.withResolvers();
  const liberee = Promise.withResolvers();
  const chemin = "**/api/v1/views/reception";
  const retenir = async (route) => {
    requetes++;
    arrivee.resolve();
    await barrage;
    try {
      await route.continue();
      const reponse = await route.request().response();
      assert(reponse?.ok(), "L’actualisation de la Réception doit réussir.");
      await reponse.finished();
      liberee.resolve();
    } catch (erreur) {
      liberee.reject(erreur);
    }
  };
  await page.route(chemin, retenir);
  try {
    if (commande === "historique") {
      await page.goBack();
    } else {
      await declencheur.click();
      await menu.getByRole("link", { name: "Réception", exact: true }).click();
    }
    await page.waitForFunction(
      (id) => document.activeElement?.getAttribute("data-fil") === id,
      avant.id,
    );
    const apres = await page.evaluate(() => ({
      section: document.activeElement.closest("section").dataset.section,
      scroll: document.querySelector("main").scrollTop,
      deborde: document.documentElement.scrollWidth > innerWidth,
    }));
    assert.equal(apres.section, avant.section);
    assert(
      Math.abs(apres.scroll - avant.scroll) <= 1,
      `Retour par ${commande} à ${apres.scroll} px au lieu de ${avant.scroll} px.`,
    );
    assert.equal(apres.deborde, false);
    const delai = setTimeout(
      () => arrivee.reject(new Error("Actualisation HTTP absente.")),
      5000,
    );
    try {
      await arrivee.promise;
    } finally {
      clearTimeout(delai);
    }
    assert.equal(requetes, 1);
  } finally {
    reprendre();
    if (requetes) await liberee.promise;
    await page.unroute(chemin, retenir);
  }
}

async function parcourir(base) {
  const browser = await ouvrirNavigateur();
  try {
    const page = await browser.newPage();
    page.setDefaultTimeout(5500);
    await connecter(page, base, "alice@recette.test", "alice-test");
    let retours = 0;
    for (const taille of tailles) {
      await verifier(page, taille);
      retours++;
    }
    if (process.env.PLI_MESURE_RETOUR === "1") await mesurerRetours(page);
    const vide = await browser.newPage();
    vide.setDefaultTimeout(5500);
    await connecter(vide, base, "bob@recette.test", "bob-test");
    const tri = vide.getByRole("button", { name: /Nouveaux expéditeurs/ });
    const compteurVide = Number(await tri.locator("strong").innerText());
    for (const taille of tailles) {
      await vide.setViewportSize(taille);
      await vide.getByText("Aucun nouveau fil.", { exact: true }).waitFor();
      await capture(vide, "vide", taille);
    }
    console.log(JSON.stringify({ retours, compteurVide }));
  } finally {
    await browser.close();
  }
}

async function mesurerRetours(page) {
  const mesures = [];
  const menu = page.getByRole("button", { name: "Menu", exact: true });
  for (let numero = 0; numero < 110; numero++) {
    await menu.click();
    await page.getByRole("link", { name: "Flux", exact: true }).click();
    await page.getByRole("heading", { name: "Flux", exact: true }).waitFor();
    await menu.click();
    const lien = page
      .getByRole("dialog")
      .getByRole("link", { name: "Réception", exact: true });
    const duree = await lien.evaluate(
      (element) =>
        new Promise((resolve, reject) => {
          const debut = performance.now();
          element.click();
          function visible() {
            if (
              document.querySelector("main [data-fil]")?.getClientRects().length
            ) {
              // La frame qui suit la mise en page laisse le navigateur peindre les fils.
              requestAnimationFrame(() => resolve(performance.now() - debut));
            } else if (performance.now() - debut > 1500)
              reject(new Error("Aucun fil rendu."));
            else requestAnimationFrame(visible);
          }
          requestAnimationFrame(visible);
        }),
    );
    if (numero >= 10) mesures.push(duree);
  }
  mesures.sort((a, b) => a - b);
  if (captures) {
    writeFileSync(
      join(captures, "mesure-retour.json"),
      JSON.stringify(
        {
          viewport: page.viewportSize(),
          echauffements: 10,
          retours: mesures.length,
          mediane_ms: (mesures[49] + mesures[50]) / 2,
          p95_ms: mesures[94],
          erreurs: 0,
        },
        null,
        2,
      ),
    );
  }
  console.error(
    `Retour en Réception sur 100 clics, médiane ${((mesures[49] + mesures[50]) / 2).toFixed(1)} ms, p95 ${mesures[94].toFixed(1)} ms.`,
  );
  assert(mesures[94] < 50, "Le p95 du retour en Réception atteint 50 ms.");
}

if (process.argv[2] === "--preparer") preparer();
else await parcourir(process.argv[2]);
