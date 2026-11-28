import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur } from "./navigateur.mjs";

const [base, adresse] = process.argv.slice(2);
// Lecture directe de la boîte, sans passer par le webmail.
function boite() {
  const script = fileURLToPath(new URL("./archivage.py", import.meta.url));
  return JSON.parse(
    execFileSync("python3", ["-B", script, adresse], { encoding: "utf8" }),
  );
}
const api = (chemin) => new URL(`/api/v1/${chemin}`, base).href;

const navigateur = await ouvrirNavigateur();
try {
  const page = await navigateur.newPage({
    viewport: { width: 1280, height: 900 },
  });
  page.setDefaultTimeout(9000);
  if (process.env.PLI_TRACE_ARCHIVAGE) {
    // Mêmes conditions de réseau que la mesure du retour en Réception.
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Network.enable");
    await cdp.send("Network.emulateNetworkConditions", {
      offline: false,
      latency: 80,
      downloadThroughput: 10_000_000 / 8,
      uploadThroughput: 2_000_000 / 8,
    });
  }
  await connecter(page, base, adresse, adresse);
  const ligne = (objet) => page.locator("main [data-fil]", { hasText: objet });
  const section = (nom, objet) =>
    page.locator(`[data-section="${nom}"] [data-fil]`, { hasText: objet });
  const coche = (objet) =>
    page.getByRole("checkbox", { name: `Sélectionner ${objet}`, exact: true });
  const commande = page.getByRole("button", {
    name: "Archiver la sélection",
    exact: true,
  });
  assert.deepEqual(boite(), {
    reception: ["Fil A", "Fil B", "Fil C"],
    lues: [],
    archives: [],
  });

  // Le fil A est consulté, puis la liste revient avec lui dans « Déjà vus ».
  // La commande d’archivage du fil charge au passage le catalogue des dossiers.
  const catalogue = page.waitForResponse(
    (r) => new URL(r.url()).pathname === "/api/v1/folders",
  );
  const lecture = page.waitForResponse((r) => r.url().endsWith("/open"));
  await ligne("Fil A").click();
  assert.equal((await lecture).status(), 200);
  assert.equal((await catalogue).status(), 200);
  await page.getByRole("button", { name: "Retour" }).click();
  await section("deja_vus", "Fil A").waitFor();

  await coche("Fil B").check();
  await coche("Fil C").check();
  await commande.waitFor();
  if (process.env.PLI_CAPTURE_ARCHIVAGE)
    await page.screenshot({
      path: process.env.PLI_CAPTURE_ARCHIVAGE.replace(".png", "-avant.png"),
    });

  // L’envoi reste retenu : la liste ne peut refléter que le geste local.
  const retenus = [];
  await page.route("**/api/v1/operations", async (route) => {
    const feu = Promise.withResolvers();
    retenus.push({ requete: route.request(), feu });
    await feu.promise;
    await route.continue();
  });
  const geste = await commande.evaluate((bouton) => {
    const presentes = () =>
      Array.from(document.querySelectorAll("main [data-fil]")).map(
        (el) => el.querySelector("[id^='objet-']").textContent,
      );
    const avant = presentes();
    const debut = performance.now();
    bouton.click();
    // Les lignes ont quitté le document avant la fin du clic.
    const apres = presentes();
    return { avant, apres, duree: performance.now() - debut };
  });
  // Les trois lettres sont arrivées à la même seconde : leur ordre n’est pas fixé.
  assert.deepEqual(geste.avant.toSorted(), ["Fil A", "Fil B", "Fil C"]);
  const immediats = geste.avant.length - geste.apres.length;
  assert.deepEqual(geste.apres, ["Fil A"]);
  assert(geste.duree < 50, `Réaction locale en ${geste.duree} ms.`);
  await page.getByText("Archivage en attente pour 2 fils.").waitFor();
  const consultes = await section("deja_vus", "Fil A")
    .filter({ hasText: "Lu" })
    .count();
  if (process.env.PLI_CAPTURE_ARCHIVAGE) {
    // Le signe d’attente se montre après un court délai.
    await page.waitForTimeout(800);
    await page.screenshot({ path: process.env.PLI_CAPTURE_ARCHIVAGE });
  }
  assert.equal(retenus.length, 1);
  const demande = retenus[0].requete.postDataJSON();
  assert.equal(demande.type, "archiver");
  assert.equal(demande.vue, "reception");
  assert.equal(demande.cibles.length, 2);
  assert.deepEqual(boite().archives, []);

  // La messagerie range B et laisse C sur place : seul C revient, sélectionné.
  retenus[0].feu.resolve();
  assert.equal((await retenus[0].requete.response()).status(), 200);
  await page.unrouteAll({ behavior: "wait" });
  await page
    .getByRole("alert")
    .filter({ hasText: "Modification non confirmée pour 1 fil." })
    .waitFor();
  await section("nouveaux", "Fil C").waitFor();
  assert.equal(await ligne("Fil B").count(), 0);
  assert.equal(await page.getByText(/Archivage en attente/).count(), 0);
  const revenus = await page
    .locator("main li input[type=checkbox]:checked")
    .count();
  assert(await coche("Fil C").isChecked());
  const range = boite();
  assert.deepEqual(range.reception, ["Fil A", "Fil C"]);
  const confirmes = range.archives.length;
  assert.deepEqual(range.archives, ["Fil B"]);

  // Rejouer la même opération rend l’état connu, sans second déplacement.
  const rejeu = await page.request.post(api("operations"), { data: demande });
  const connu = await page.request.get(api(`operations/${demande.id}`));
  assert.equal(rejeu.status(), 200);
  const resultat = await rejeu.json();
  assert.equal(resultat.etat, "confirmee");
  assert.deepEqual(
    resultat.resultats.map((cible) => cible.etat).sort(),
    ["confirme", "refuse"],
  );
  const identiques = Number(
    JSON.stringify(resultat) === JSON.stringify(await connu.json()),
  );
  assert.deepEqual(boite(), range);

  // La reprise ne porte que sur le fil proposé : cette fois la messagerie le range.
  // Le clic, la disparition de la ligne et la confirmation partagent l’horloge de la page.
  const reprise = await commande.evaluate(
    (bouton) =>
      new Promise((resoudre) => {
        const zone = document.querySelector("main");
        const trace = {
          origine_utc: new Date(performance.timeOrigin).toISOString(),
          clic: performance.now(),
        };
        const finir = () => {
          if (trace.rendu_contenu && trace.confirmation_distante)
            resoudre(trace);
        };
        const temoin = new MutationObserver(() => {
          if (zone.textContent.includes("Archivage en attente")) return;
          temoin.disconnect();
          trace.confirmation_distante = performance.now();
          finir();
        });
        bouton.click();
        trace.lignes = zone.querySelectorAll("[data-fil]").length;
        temoin.observe(zone, {
          subtree: true,
          childList: true,
          characterData: true,
        });
        // À l’image suivante, la liste est affichée sans la ligne archivée.
        requestAnimationFrame(() => {
          trace.rendu_contenu = performance.now();
          finir();
        });
      }),
  );
  assert.equal(reprise.lignes, 1);
  await ligne("Fil C").waitFor({ state: "detached" });
  await page.waitForFunction(
    () =>
      !document.querySelector("[role=alert]") &&
      !document.querySelector("main")?.textContent.includes("en attente"),
  );
  await section("deja_vus", "Fil A").waitFor();
  const final = boite();
  assert.deepEqual(final.archives, ["Fil B", "Fil C"]);
  // Le fil A, seulement lu, n’a pas bougé.
  assert.deepEqual(final.reception, ["Fil A"]);
  assert.deepEqual(final.lues, ["Fil A"]);
  if (process.env.PLI_TRACE_ARCHIVAGE) {
    const { lignes: _lignes, ...horodatages } = reprise;
    const trace = {
      version: 1,
      parcours: "archiver",
      ...horodatages,
      conditions: {
        jeu: "trois fils",
        cache: "chaud",
        fournisseur: "GreenMail 2.1.15",
        navigateur: `Chromium ${navigateur.version()}`,
        appareil: "bureau",
        fenetre_px: [1280, 900],
        reseau: { reception_mbit_s: 10, envoi_mbit_s: 2, rtt_ms: 80 },
      },
      mesures: { cpu_navigateur_ms: geste.duree },
      phase: "mesure",
      resultat: "ok",
    };
    appendFileSync(
      process.env.PLI_TRACE_ARCHIVAGE,
      `${JSON.stringify(trace)}\n`,
    );
  }
  console.log(
    JSON.stringify({
      immediats,
      consultes,
      confirmes,
      revenus,
      identiques,
      archives: final.archives.length,
      deplaces: final.archives.filter((objet) => objet === "Fil A").length,
    }),
  );
} finally {
  await navigateur.close();
}
