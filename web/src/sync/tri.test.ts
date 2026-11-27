import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import donnees from "../../tests/expediteur-tri.json";
import receptionVide from "../../tests/reception-vide.json";
import { amorcerVue, projeterTri } from "./projection-tri";
import { creerTri } from "./tri";
import {
  type Expediteur,
  envoyerDecision,
  type IntentionTri,
  lireCatalogue,
  lireLettres,
} from "./tri-api";
import { creerCourrier, type Fil, type PageFils } from "./vues";

const entree = donnees as Expediteur;
const ancien = entree.apercus[1];
const courante: PageFils = {
  ...receptionVide,
  expediteurs_en_attente: 1,
  deja_vus: [ancien],
};
const intention: IntentionTri = {
  entree,
  choix: "accepte",
  destination: "flux",
};
const autre: Fil = {
  ...ancien,
  id: "Fil personnel",
  expediteur: { nom: "Noa", adresse: "noa@recette.test" },
};
const catalogue = (
  expediteurs: Expediteur[],
  suite: string | null = null,
  revision = 2,
) => ({ compte: 3, revision, expediteurs, suite });
const enregistree = (etat: "accepte" | "refuse", destination = "flux") => ({
  compte: 3,
  revision_compte: 8,
  decision: {
    adresse: entree.adresse,
    etat,
    destination,
    revision: 2,
    date_decision: 777,
  },
});

function reponseRetenue() {
  let resolve!: (reponse: Response) => void;
  const promise = new Promise<Response>((terminer) => {
    resolve = terminer;
  });
  return { promise, resolve };
}

it("masque l’ancien courrier et affiche les fils dans Flux alors que la décision attend le serveur", async () => {
  const demande = reponseRetenue();
  const ancienGet = reponseRetenue();
  const pageApres = reponseRetenue();
  const lireReception = vi
    .fn()
    .mockResolvedValueOnce(Response.json(courante))
    .mockReturnValueOnce(ancienGet.promise)
    .mockReturnValueOnce(pageApres.promise);
  const requete = vi.fn((url: RequestInfo | URL, init?: RequestInit) => {
    if (url === "/api/v1/views/reception") return lireReception();
    if (url === "/api/v1/views/flux")
      return Promise.resolve(
        Response.json({
          ...courante,
          vue: "flux",
          revision: 8,
          deja_vus: [ancien],
          nouveaux: [entree.apercus[0]],
          expediteurs_en_attente: 0,
        }),
      );
    if (url === "/api/v1/screener/decisions" && init?.method === "POST")
      return demande.promise;
    throw new Error(String(url));
  });
  vi.stubGlobal("fetch", requete);
  const courrier = creerCourrier(3);
  await courrier.chargerVue("reception");
  const retourAncien = courrier.chargerVue("reception");
  const reception = courrier.obtenirVue("reception");
  const changement = courrier.tri.accepterExpediteur(entree, "flux");
  await courrier.tri.refuserExpediteur(entree);
  flush();
  expect(reception.etat()).toMatchObject({
    statut: "pret",
    valeur: { deja_vus: [], expediteurs_en_attente: 0 },
  });
  const flux = courrier.obtenirVue("flux");
  expect(flux.etat()).toMatchObject({
    statut: "pret",
    valeur: { vue: "flux", nouveaux: [entree.apercus[0]], deja_vus: [ancien] },
  });
  expect(courrier.obtenirVue("justificatifs").etat()).toEqual({
    statut: "chargement",
  });
  const posts = requete.mock.calls.filter(
    ([u]) => u === "/api/v1/screener/decisions",
  );
  expect(posts).toHaveLength(1);
  expect(JSON.parse(String(posts[0][1]?.body))).toEqual({
    adresse: entree.adresse,
    decision: "accepte",
    destination: "flux",
    revision_attendue: 1,
  });
  demande.resolve(Response.json(enregistree("accepte")));
  await changement;
  expect(courrier.tri.intentions()).toEqual([]);
  expect(courrier.tri.avis()).toBeUndefined();
  ancienGet.resolve(Response.json(courante));
  await retourAncien;
  expect(reception.etat()).toMatchObject({
    statut: "pret",
    valeur: { deja_vus: [], expediteurs_en_attente: 0 },
  });
  expect(lireReception).toHaveBeenCalledTimes(3);
  pageApres.resolve(
    Response.json({
      ...courante,
      revision: 8,
      deja_vus: [],
      expediteurs_en_attente: 0,
    }),
  );
  await reception.charger();
  expect(reception.etat()).toMatchObject({ valeur: { revision: 8 } });
  expect(flux.etat()).toMatchObject({ valeur: { revision: 8 } });
});

it("annule l’aperçu après une coupure et représente la décision enregistrée en cas de conflit", async () => {
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(catalogue([entree])))
    .mockResolvedValueOnce(Response.json(catalogue([])));
  vi.stubGlobal("fetch", source);
  const confirmer = vi.fn();
  const tri = creerTri(3, confirmer);
  await tri.attente.charger();
  await tri.historique.charger();
  source.mockRejectedValueOnce(new Error("Coupure"));
  await tri.accepterExpediteur(entree);
  expect(tri.avis()).toEqual({ type: "erreur", adresse: entree.adresse });
  expect(tri.attente.etat()).toMatchObject({
    valeur: { expediteurs: [entree] },
  });
  expect(confirmer).not.toHaveBeenCalled();
  source.mockResolvedValueOnce(
    Response.json(enregistree("refuse", "reception"), { status: 409 }),
  );
  await tri.accepterExpediteur(entree);
  expect(tri.avis()).toMatchObject({
    type: "conflit",
    entree: { etat: "refuse", revision: 2, destination: "reception" },
  });
  expect(confirmer).toHaveBeenCalledWith(
    { ...intention, choix: "refuse", destination: "reception" },
    8,
  );
  expect(tri.attente.etat()).toMatchObject({ valeur: { expediteurs: [] } });
  expect(tri.historique.etat()).toMatchObject({
    valeur: {
      expediteurs: [{ adresse: entree.adresse, etat: "refuse", revision: 2 }],
    },
  });
  tri.fermerAvis();
  flush();
  expect(tri.avis()).toBeUndefined();
  const conflit = { ...entree, etat: "refuse" as const, revision: 2 };
  source.mockResolvedValueOnce(
    Response.json({
      ...enregistree("accepte"),
      revision_compte: 9,
      decision: { ...enregistree("accepte").decision, revision: 3 },
    }),
  );
  await tri.accepterExpediteur(conflit, "flux");
  expect(JSON.parse(source.mock.lastCall?.[1].body).revision_attendue).toBe(2);
  expect(tri.historique.etat()).toMatchObject({
    valeur: { expediteurs: [{ etat: "accepte", revision: 3 }] },
  });
});

it("filtre uniquement Réception et ses vues et conserve les indicateurs d’un fil partagé", () => {
  const partage: Fil = {
    ...autre,
    id: ancien.id,
    date: 1,
    nombre_messages: 3,
    etoile: true,
    repondu: true,
    transfere: true,
    piece_jointe: true,
  };
  const avant = { ...courante, vue: "flux", deja_vus: [partage, autre] };
  const apres = projeterTri(avant, "flux", intention, false);
  expect(apres.deja_vus).toContainEqual({
    ...ancien,
    nombre_messages: 4,
    etoile: true,
    repondu: true,
    transfere: true,
    piece_jointe: true,
  });
  expect(apres.deja_vus).toContainEqual(autre);
  expect(apres.nouveaux).toEqual([entree.apercus[0]]);
  expect(
    projeterTri(
      courante,
      "dossier/2",
      { ...intention, choix: "refuse" },
      false,
    ),
  ).toMatchObject({ deja_vus: [ancien], expediteurs_en_attente: 0 });
  expect(
    projeterTri(courante, "dossier/1", { ...intention, choix: "refuse" }, true)
      .deja_vus,
  ).toEqual([]);
  expect(amorcerVue(3, "dossier/1", [intention], true, 4)).toMatchObject({
    compte: 3,
    dossier: 1,
    expediteurs_en_attente: 4,
  });
  expect(amorcerVue(3, "dossier/1", [], true, 4)).toBeUndefined();
});

it("vérifie le compte, l’adresse et la révision avant d’appliquer une réponse de tri", async () => {
  for (const corps of [
    null,
    { compte: 3, expediteurs: null },
    { compte: 5, expediteurs: [] },
  ]) {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValueOnce(Response.json(corps)));
    await expect(lireCatalogue(3, "attente")).rejects.toThrow(
      "Catalogue de tri illisible.",
    );
  }
  for (const corps of [null, { messages: false }]) {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValueOnce(Response.json(corps)));
    await expect(
      lireLettres(entree.adresse, { uid: 21, generation: 1 }),
    ).rejects.toThrow("Courrier de l’expéditeur illisible.");
  }
  for (const corps of [
    null,
    { ...enregistree("accepte"), compte: 4 },
    { ...enregistree("accepte"), decision: null },
    { ...enregistree("accepte"), decision: { adresse: "autre@recette.test" } },
    { ...enregistree("accepte"), revision_compte: "7" },
  ]) {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValueOnce(Response.json(corps)));
    await expect(envoyerDecision(3, intention)).rejects.toThrow(
      "Décision de tri illisible.",
    );
  }
});

it("charge les courriers et les adresses suivantes dans leur propre catalogue", async () => {
  const marque = { uid: 20, generation: 2 };
  const premiere = { ...entree, suivant: marque };
  const b = { ...entree, adresse: "bois@recette.test" };
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(catalogue([premiere], entree.adresse)))
    .mockResolvedValueOnce(
      Response.json({
        messages: [{ ...entree.messages[0], uid: 7 }],
        suivant: null,
      }),
    )
    .mockResolvedValueOnce(Response.json(catalogue([b])));
  vi.stubGlobal("fetch", source);
  const tri = creerTri(3, vi.fn());
  await tri.suite("historique");
  await tri.historique.charger();
  const courant = tri.historique.etat();
  if (courant.statut !== "pret") throw new Error("Catalogue absent");
  await tri.lettresSuivantes("historique", courant.valeur.expediteurs[0]);
  await tri.suite("historique");
  const fin = tri.historique.etat();
  if (fin.statut !== "pret") throw new Error("Catalogue absent");
  expect(fin.valeur.expediteurs.map((e) => e.adresse)).toEqual([
    entree.adresse,
    b.adresse,
  ]);
  expect(fin.valeur.expediteurs[0].messages.at(-1)?.uid).toBe(7);
  expect(fin.valeur.expediteurs[0].suivant).toBeNull();
  expect(fin.valeur.suite).toBeNull();
  await tri.lettresSuivantes("historique", fin.valeur.expediteurs[0]);
  await tri.suite("historique");
  expect(source.mock.calls.map(([url]) => url)).toEqual([
    "/api/v1/screener/historique?details=true",
    "/api/v1/screener/messages?adresse=atelier%40recette.test&avant=20&generation=2",
    "/api/v1/screener/historique?details=true&apres=atelier%40recette.test",
  ]);
});
