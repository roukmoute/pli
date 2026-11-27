import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import donnees from "../../tests/expediteur-tri.json";
import receptionVide from "../../tests/reception-vide.json";
import { amorcerVue, comparerFils, projeterTri } from "./projection-tri";
import { ressource } from "./ressource";
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

it("préserve l’ordre date puis identifiant et les indicateurs des conversations réunies", () => {
  expect(comparerFils(ancien, { ...ancien })).toBe(0);
  const ajoute = {
    ...ancien,
    id: "commun",
    date: 11,
    non_lu: true,
    objet: "nouvelle réponse",
  };
  const precedent = { ...autre, id: "commun", date: 11, non_lu: false };
  const fils = ["b", "c", "a", "z"].map((id) => ({
    ...autre,
    id,
    date: id === "z" ? 1 : 11,
    non_lu: true,
  }));
  const avant = { ...courante, nouveaux: fils, deja_vus: [precedent] };
  const choix = { ...intention, entree: { ...entree, apercus: [ajoute] } };
  const apres = projeterTri(avant, "flux", choix, false);
  expect(apres.nouveaux.map((f) => f.id)).toEqual([
    "commun",
    "c",
    "b",
    "a",
    "z",
  ]);
  expect(apres.nouveaux[0]).toMatchObject({
    objet: "nouvelle réponse",
    nombre_messages: 2,
    non_lu: true,
  });
  const recent = {
    ...precedent,
    date: 50,
    non_lu: true,
    objet: "la dernière lettre",
  };
  expect(
    projeterTri({ ...avant, deja_vus: [recent] }, "flux", choix, false)
      .nouveaux[0],
  ).toMatchObject({ objet: "la dernière lettre" });
  const recule = projeterTri(
    { ...avant, nouveaux: [...fils].reverse() },
    "flux",
    choix,
    false,
  );
  expect(recule.nouveaux.map((f) => f.id)).toEqual(
    apres.nouveaux.map((f) => f.id),
  );
});

it("amorce une page froide avec les marqueurs initiaux et écrête le nombre d’expéditeurs", () => {
  expect(amorcerVue(3, "flux", [intention], false, 0)).toEqual({
    compte: 3,
    vue: "flux",
    nouveaux: [],
    deja_vus: [],
    revision: 0,
    expediteurs_en_attente: 0,
    curseur_suivant: null,
    couverture: { import_en_cours: false, proportion_indexee: null },
    compteurs: { reception: 0, flux: 0, justificatifs: 0 },
  });
  const vide = { ...courante, expediteurs_en_attente: 0 };
  expect(
    projeterTri(vide, "reception", intention, false).expediteurs_en_attente,
  ).toBe(0);
});

it("écarte un ancien échec de lecture et protège la photographie d’une écriture", async () => {
  const vieille = reponseRetenue();
  const fraiche = reponseRetenue();
  const obtenir = vi
    .fn()
    .mockReturnValueOnce(vieille.promise)
    .mockReturnValueOnce(fraiche.promise);
  const page = ressource(async () => {
    const reponse = await obtenir();
    if (!reponse.ok) throw new Error("Indisponible");
    return reponse.text();
  });
  const avant = page.charger();
  page.corriger((s) => s);
  const apres = page.charger();
  vieille.resolve(new Response(null, { status: 503 }));
  await avant;
  expect(page.charger()).toBe(apres);
  expect(page.etat()).toEqual({ statut: "chargement" });
  fraiche.resolve(new Response("dernière"));
  await apres;
  expect(page.etat()).toEqual({
    statut: "pret",
    valeur: "dernière",
    perime: false,
  });
  const autre = ressource(async () => 10);
  autre.corriger((n) => n + 1, 0);
  autre.corriger((n) => n * 2);
  flush();
  expect(autre.etat()).toEqual({ statut: "pret", valeur: 2, perime: false });
});

it("transmet les erreurs HTTP ordinaires et refuse une décision encore en attente", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(
        Response.json(
          { code: "non", message: "Refus de la requête" },
          { status: 400 },
        ),
      ),
  );
  await expect(envoyerDecision(3, intention)).rejects.toMatchObject({
    status: 400,
    code: "non",
  });
  const reponse = enregistree("accepte");
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValueOnce(
      Response.json({
        ...reponse,
        decision: { ...reponse.decision, etat: "en_attente" },
      }),
    ),
  );
  const confirmer = vi.fn();
  const tri = creerTri(3, confirmer);
  await tri.accepterExpediteur(entree);
  expect(tri.avis()).toEqual({ type: "erreur", adresse: entree.adresse });
  expect(confirmer).not.toHaveBeenCalled();
});

it("repart de la première page si la boîte change et ignore une pagination déjà remplacée", async () => {
  const suiteA = reponseRetenue();
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(catalogue([entree], entree.adresse)))
    .mockReturnValueOnce(suiteA.promise)
    .mockResolvedValueOnce(Response.json(catalogue([entree])))
    .mockResolvedValueOnce(
      Response.json(catalogue([entree], entree.adresse, 3)),
    )
    .mockResolvedValueOnce(Response.json(catalogue([], null, 5)))
    .mockResolvedValueOnce(Response.json(catalogue([], null, 5)));
  vi.stubGlobal("fetch", source);
  const tri = creerTri(3, vi.fn());
  await tri.attente.charger();
  const requete = tri.suite("attente");
  await tri.attente.charger();
  suiteA.resolve(
    Response.json(catalogue([{ ...entree, adresse: "ancienne@recette.test" }])),
  );
  await requete;
  expect(tri.attente.etat()).toMatchObject({
    valeur: { expediteurs: [entree], suite: null },
  });
  await tri.attente.charger();
  await tri.suite("attente");
  expect(tri.attente.etat()).toMatchObject({
    valeur: { expediteurs: [], revision: 5 },
  });
  expect(source.mock.lastCall?.[0]).toBe("/api/v1/screener?details=true");
});

it("conserve le curseur d’une autre adresse et écarte les lettres d’une page remplacée", async () => {
  const premiere = { ...entree, suivant: { uid: 4, generation: 1 } };
  const autre = { ...premiere, adresse: "verger@recette.test" };
  const lot = reponseRetenue();
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(catalogue([premiere, autre])))
    .mockReturnValueOnce(lot.promise)
    .mockResolvedValueOnce(
      Response.json(catalogue([{ ...premiere, suivant: null }, autre])),
    );
  vi.stubGlobal("fetch", source);
  const tri = creerTri(3, vi.fn());
  await tri.attente.charger();
  const page = tri.attente.etat();
  if (page.statut !== "pret") throw new Error("Catalogue absent");
  const requete = tri.lettresSuivantes("attente", page.valeur.expediteurs[0]);
  await tri.attente.charger();
  lot.resolve(
    Response.json({
      messages: [{ ...entree.messages[0], uid: 1 }],
      suivant: null,
    }),
  );
  await requete;
  expect(tri.attente.etat()).toMatchObject({
    valeur: { expediteurs: [{ ...premiere, suivant: null }, autre] },
  });
});

it("insère une décision au rang de son adresse et remplace son ancienne entrée", async () => {
  const avant = { ...entree, adresse: "abeille@recette.test" };
  const fin = { ...entree, adresse: "verger@recette.test" };
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(catalogue([avant, fin])))
    .mockResolvedValueOnce(Response.json(enregistree("accepte")))
    .mockResolvedValueOnce(
      Response.json({
        ...enregistree("refuse"),
        decision: { ...enregistree("refuse").decision, revision: 3 },
      }),
    )
    .mockResolvedValueOnce(
      Response.json({
        ...enregistree("accepte"),
        decision: {
          ...enregistree("accepte").decision,
          adresse: avant.adresse,
        },
      }),
    );
  vi.stubGlobal("fetch", source);
  const tri = creerTri(3, vi.fn());
  await tri.historique.charger();
  await tri.accepterExpediteur(entree, "flux");
  await tri.refuserExpediteur({ ...entree, revision: 2 });
  await tri.accepterExpediteur(avant);
  const e = tri.historique.etat();
  if (e.statut !== "pret") throw new Error("Historique absent");
  expect(e.valeur.expediteurs.map((exp) => exp.adresse)).toEqual([
    avant.adresse,
    entree.adresse,
    fin.adresse,
  ]);
  expect(e.valeur.expediteurs[1]).toMatchObject({
    etat: "refuse",
    revision: 3,
  });
});

it("identifie INBOX même en ouvrant le tri par un lien et préserve les autres dossiers en cache", async () => {
  const post = reponseRetenue();
  const dossiers = {
    folders: [
      { id: 6, path: "INBOX", role: "inbox", hidden: false },
      { id: 7, path: "Junk", role: "junk", hidden: false },
    ],
  };
  const source = vi.fn(async (url: RequestInfo | URL) => {
    if (url === "/api/v1/folders") return Response.json(dossiers);
    if (url === "/api/v1/screener/decisions") return post.promise;
    if (url === "/api/v1/folders/7/messages")
      return Response.json({ ...courante, vue: undefined, dossier: 7 });
    throw new Error(String(url));
  });
  vi.stubGlobal("fetch", source);
  const courrier = creerCourrier(3);
  await courrier.dossiers.charger();
  const spam = courrier.obtenirVue("dossier/7");
  await spam.charger();
  const action = courrier.tri.accepterExpediteur(entree, "flux");
  flush();
  const inbox = courrier.obtenirVue("dossier/6");
  expect(inbox.etat()).toMatchObject({
    statut: "pret",
    perime: false,
    valeur: {
      dossier: 6,
      expediteurs_en_attente: 0,
      nouveaux: [entree.apercus[0]],
    },
  });
  expect(spam.etat()).toMatchObject({
    valeur: { deja_vus: [ancien], nouveaux: [] },
  });
  expect(courrier.obtenirVue("dossier/9").etat()).toEqual({
    statut: "chargement",
  });
  post.resolve(Response.json(enregistree("accepte")));
  await action;
  expect(
    source.mock.calls.filter(([url]) => url === "/api/v1/folders/7/messages"),
  ).toHaveLength(1);
  expect(source.mock.calls.map(([url]) => url)).toContain("/api/v1/views/flux");
  expect(source.mock.calls.map(([url]) => url)).toContain(
    "/api/v1/folders/6/messages",
  );
});

it("garde le compteur d’une page déjà à la révision du serveur après un conflit", async () => {
  const post = reponseRetenue();
  const rafraichir = reponseRetenue();
  const initiale = { ...courante, revision: 8, expediteurs_en_attente: 3 };
  const source = vi
    .fn()
    .mockResolvedValueOnce(Response.json(initiale))
    .mockRejectedValueOnce(new Error("Réseau interrompu"))
    .mockReturnValueOnce(post.promise)
    .mockReturnValueOnce(rafraichir.promise);
  vi.stubGlobal("fetch", source);
  const courrier = creerCourrier(3);
  const reception = courrier.obtenirVue("reception");
  await reception.charger();
  await reception.charger();
  const action = courrier.tri.refuserExpediteur(entree);
  flush();
  expect(reception.etat()).toMatchObject({
    statut: "pret",
    perime: true,
    valeur: { expediteurs_en_attente: 2 },
  });
  post.resolve(
    Response.json(enregistree("refuse", "reception"), { status: 409 }),
  );
  await action;
  expect(reception.etat()).toMatchObject({
    statut: "pret",
    perime: true,
    valeur: initiale,
  });
  rafraichir.resolve(Response.json(initiale));
  await reception.charger();
  expect(reception.etat()).toMatchObject({ perime: false });
});

it("ouvre directement la révision d’une adresse refusée sans inventer de compteur", async () => {
  const reponse = reponseRetenue();
  const source = vi
    .fn()
    .mockReturnValueOnce(reponse.promise)
    .mockResolvedValueOnce(
      Response.json({ ...courante, vue: "flux", expediteurs_en_attente: 0 }),
    );
  vi.stubGlobal("fetch", source);
  const courrier = creerCourrier(3);
  const refusee = { ...entree, etat: "refuse" as const, revision: 2 };
  const action = courrier.tri.accepterExpediteur(refusee, "flux");
  flush();
  const flux = courrier.obtenirVue("flux");
  expect(flux.etat()).toMatchObject({
    statut: "pret",
    perime: false,
    valeur: { expediteurs_en_attente: 0 },
  });
  reponse.resolve(Response.json(enregistree("accepte")));
  await action;
  expect(courrier.tri.avis()).toBeUndefined();
});
