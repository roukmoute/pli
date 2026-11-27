import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { retour } from "../bench/observer-retour.mjs";

export async function mesurerRetours(page, navigateur, ouvrirFil) {
  const dossier = fileURLToPath(new URL("../bench/traces/e2e/", import.meta.url));
  const traces = [];
  const cdp = await page.context().newCDPSession(page);
  const origine = await page.evaluate(() => performance.timeOrigin);
  const departsHttp = [];
  cdp.on("Network.requestWillBeSent", ({ request, wallTime }) => {
    if (/^https?:/.test(request.url)) departsHttp.push(wallTime * 1000 - origine);
  });
  await cdp.send("Network.enable");
  await cdp.send("Network.emulateNetworkConditions", {
    offline: false, latency: 80, downloadThroughput: 10_000_000 / 8, uploadThroughput: 2_000_000 / 8,
  });
  const conditions = {
    jeu: "base", graine: 42, anciens_ajoutes: 31, cache: "chaud", fournisseur: "GreenMail 2.1.15",
    navigateur: "Chromium " + navigateur.version(), appareil: "bureau",
    fenetre_px: [page.viewportSize().width, page.viewportSize().height],
    reseau: { reception_mbit_s: 10, envoi_mbit_s: 2, rtt_ms: 80 },
  };
  for (let i = 0; i < 110; i++) {
    await ouvrirFil();
    const mesure = await retour(page.getByRole("button", { name: /Retour/ }));
    traces.push({
      version: 1, parcours: "retour_reception", ...mesure, confirmation_distante: null,
      conditions, phase: i < 10 ? "echauffement" : "mesure",
    });
  }
  await cdp.detach();
  for (const t of traces) {
    const http = departsHttp.filter((debut) => debut >= t.clic && debut < t.rendu_contenu).length;
    t.requetes_avant_rendu = Math.max(t.requetes_avant_rendu, http);
    t.resultat = t.requetes_avant_rendu ? "erreur" : "ok";
    if (t.requetes_avant_rendu) t.code_erreur = "requete_anticipee";
  }
  const retenues = traces.filter((t) => t.phase === "mesure");
  const durees = retenues.map((t) => t.rendu_contenu - t.clic).sort((a, b) => a - b);
  const resume = {
    echauffements: 10, repetitions: durees.length, mediane_ms: (durees[49] + durees[50]) / 2,
    p95_ms: durees[94], requetes_avant_rendu: retenues.reduce((n, t) => n + t.requetes_avant_rendu, 0),
  };
  await mkdir(dossier, { recursive: true });
  await writeFile(join(dossier, "retour-reception.jsonl"), traces.map((t) => JSON.stringify(t)).join("\n") + "\n");
  await writeFile(join(dossier, "retour-reception.json"), JSON.stringify(resume, null, 2) + "\n");
  console.log("Retour du fil à la Réception", resume);
  assert.equal(resume.requetes_avant_rendu, 0, "Une requête a précédé le rendu de la liste en cache.");
  assert(resume.p95_ms < 50, "Le retour atteint 50 ms au 95e percentile.");
}
