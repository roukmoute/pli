import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import { reponseRetenue } from "../../tests/reponse-retenue";
import { lireDossiers, lirePage } from "./pages-api";
import { creerCourrier, type PageFils, type Vue } from "./vues";

const premiere = {
  compte: 5,
  couverture: { import_en_cours: false, proportion_indexee: 1 },
  vue: "reception",
  nouveaux: [],
  deja_vus: [],
  expediteurs_en_attente: 3,
  revision: 4,
};

it("partage une requête en cours et conserve la dernière page en cas de coupure", async () => {
  let finir!: (value: Response) => void;
  const requete = vi.fn<typeof fetch>().mockReturnValueOnce(
    new Promise((resolve) => {
      finir = resolve;
    }),
  );
  vi.stubGlobal("fetch", requete);
  const courrier = creerCourrier(5);
  const entree = courrier.obtenirVue("reception");
  expect(entree.repere).toEqual({
    defilement: 0,
    section: "nouveaux",
    ordre: "date",
    filtre: "tous",
  });
  const premiereLecture = courrier.chargerVue("reception");
  expect(courrier.chargerVue("reception")).toBe(premiereLecture);
  expect(entree.etat()).toEqual({ statut: "chargement" });
  finir(Response.json(premiere));
  await premiereLecture;
  expect(entree.etat()).toEqual({
    statut: "pret",
    valeur: premiere,
    perime: false,
  });
  entree.repere.fil = "lettre-conservée";
  requete.mockRejectedValueOnce(new Error("Réseau coupé"));
  const enFond = courrier.chargerVue("reception");
  expect(entree.etat()).toEqual({
    statut: "pret",
    valeur: premiere,
    perime: false,
  });
  await enFond;
  expect(entree.etat()).toEqual({
    statut: "pret",
    valeur: premiere,
    perime: true,
  });
  requete.mockResolvedValueOnce(Response.json({ ...premiere, revision: 8 }));
  await courrier.chargerVue("reception");
  expect(entree.etat()).toMatchObject({
    statut: "pret",
    valeur: { revision: 8 },
    perime: false,
  });
  expect(courrier.obtenirVue("reception").repere.fil).toBe("lettre-conservée");
  expect(courrier.obtenirVue("flux").repere.fil).toBeUndefined();
  expect(creerCourrier(7).obtenirVue("reception").etat()).toEqual({
    statut: "chargement",
  });
  expect(requete).toHaveBeenCalledTimes(3);
});

it.each([
  ["reception", null],
  ["reception", { ...premiere, couverture: null }],
  ["reception", { ...premiere, expediteurs_en_attente: "2" }],
  ["reception", { ...premiere, compte: 2 }],
  ["reception", { ...premiere, vue: "flux" }],
  ["reception", { ...premiere, nouveaux: null }],
  ["reception", { ...premiere, deja_vus: null }],
  ["dossier/4", { ...premiere, dossier: 5 }],
] as const)("rejette la réponse incompatible pour %s", async (vue, corps) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(corps)));
  const compte = creerCourrier(5);
  await compte.chargerVue(vue);
  expect(compte.obtenirVue(vue).etat()).toEqual({ statut: "erreur" });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(corps)));
  await expect(lirePage(5, vue)).rejects.toThrow(
    "La liste reçue ne correspond pas à la boîte ouverte.",
  );
});

it("redemande une liste en erreur et isole un dossier physique d’une vue logique", async () => {
  const dossiers = creerCourrier(5);
  const vue: Vue = "dossier/42";
  const source = vi
    .fn()
    .mockRejectedValueOnce(new Error("Hors ligne"))
    .mockResolvedValueOnce(
      Response.json({ ...premiere, vue: undefined, dossier: 42 }),
    );
  vi.stubGlobal("fetch", source);
  await dossiers.chargerVue(vue);
  expect(dossiers.obtenirVue(vue).etat()).toEqual({ statut: "erreur" });
  const reprise = dossiers.chargerVue(vue);
  flush();
  expect(dossiers.obtenirVue(vue).etat()).toEqual({ statut: "chargement" });
  await reprise;
  expect(source).toHaveBeenCalledWith(
    "/api/v1/folders/42/messages",
    expect.any(Object),
  );
  expect(dossiers.obtenirVue(vue).etat()).toMatchObject({
    statut: "pret",
    valeur: { dossier: 42 },
  });
});

it("écarte les dossiers masqués et permet de reprendre un catalogue illisible", async () => {
  const compte = creerCourrier(5);
  for (const corps of [null, { folders: false }]) {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(corps)));
    await compte.dossiers.charger();
    expect(compte.dossiers.etat()).toEqual({ statut: "erreur" });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(corps)));
    await expect(lireDossiers()).rejects.toThrow(
      "Liste de dossiers illisible.",
    );
  }
  const visible = { id: 8, path: "Verger", role: null, hidden: false };
  const masque = { id: 9, path: "Masqué", role: null, hidden: true };
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(Response.json({ folders: [visible, masque] })),
  );
  await compte.dossiers.charger();
  expect(compte.dossiers.etat()).toEqual({
    statut: "pret",
    valeur: [visible],
    perime: false,
  });
});

it("montre l’action avant la réponse puis relit les vues qui présentent la cible", async () => {
  const courrier = creerCourrier(5);
  const b = fil("b", 10);
  // Le serveur garde l’état du fil et confirme ou refuse chaque opération.
  const serveur = { fil: b, etat: "confirme", revision: 4 };
  const retenue = reponseRetenue<void>();
  let reponse = retenue.promise;
  const source = vi.fn(async (url: string, init: RequestInit) => {
    if (url.includes("/threads/")) return Response.json([]);
    if (url.includes("folders/"))
      return Response.json({
        ...premiere,
        vue: undefined,
        dossier: 8,
        deja_vus: [{ id: "b" }],
      });
    if (!url.endsWith("/operations"))
      return Response.json({
        ...premiere,
        vue: url.split("/").at(-1),
        nouveaux: url.endsWith("/reception") ? [serveur.fil] : [],
        revision: serveur.revision,
      });
    await reponse;
    const demande = JSON.parse(String(init.body));
    if (serveur.etat === "confirme") {
      serveur.fil = { ...serveur.fil, etoile: demande.type === "etoile" };
      serveur.revision++;
    }
    return Response.json({
      ...demande,
      etat: serveur.etat === "confirme" ? "confirmee" : "refusee",
      revision: serveur.revision,
      resultats: [{ id: "b", etat: serveur.etat }],
    });
  });
  vi.stubGlobal("fetch", source);
  const entree = courrier.obtenirVue("reception");
  await Promise.all([
    entree.charger(),
    courrier.chargerVue("dossier/8"),
    courrier.chargerVue("flux"),
  ]);
  courrier.obtenirVue("justificatifs");
  const fermer = courrier.lectures.consulter("b");
  const affiche = () => {
    flush();
    const etat = entree.etat();
    return etat.statut === "pret" ? etat.valeur.nouveaux[0] : undefined;
  };
  const appels = () => source.mock.calls.map(([url]) => url).sort();
  source.mockClear();
  const etoilee = entree.selection.marquer(["b"], "etoile", true);
  // Avant toute réponse, la liste montre l’étoile et son attente.
  expect(affiche()).toEqual({ ...b, etoile: true, attente: { etoile: true } });
  // Le cache, lui, attend la confirmation.
  expect(entree.source.valeur()?.nouveaux[0]).toEqual(b);
  expect(appels()).toEqual(["/api/v1/operations"]);
  retenue.resolve();
  await etoilee;
  expect(affiche()).toEqual({ ...b, etoile: true });
  expect(appels()).toEqual([
    "/api/v1/folders/8/messages",
    "/api/v1/operations",
    "/api/v1/views/reception",
  ]);
  expect(JSON.parse(String(source.mock.calls[0][1].body))).toMatchObject({
    type: "etoile",
    vue: "reception",
    cibles: ["b"],
  });
  // Un refus rend l’état d’avant et s’annonce dans la sélection.
  serveur.etat = "refuse";
  reponse = Promise.resolve();
  source.mockClear();
  await entree.selection.marquer(["b"], "lu", true);
  expect(affiche()).toEqual({ ...b, etoile: true });
  expect(entree.selection.echecs()).toBe(1);
  expect(appels()).toEqual([
    "/api/v1/folders/8/messages",
    "/api/v1/operations",
    "/api/v1/threads/b",
    "/api/v1/views/reception",
  ]);
  fermer();
});

function fenetre(entree: { source: { valeur: () => PageFils | undefined } }) {
  return entree.source.valeur() as PageFils;
}

it("demande la tranche d’une vue ou d’un dossier avec son curseur et sa taille", async () => {
  const dossier = { ...premiere, vue: undefined, dossier: 4 };
  const requete = vi.fn(async (url: string) =>
    Response.json(url.includes("folders") ? dossier : premiere),
  );
  vi.stubGlobal("fetch", requete);
  await lirePage(5, "reception");
  await lirePage(5, "reception", { taille: 3 });
  await lirePage(5, "dossier/4", { curseur: "apres b/c", taille: 40 });
  await lirePage(5, "dossier/4", { curseur: "suite", taille: 0 });
  expect(requete.mock.calls.map(([url]) => url)).toEqual([
    "/api/v1/views/reception",
    "/api/v1/views/reception?taille=3",
    "/api/v1/folders/4/messages?curseur=apres+b%2Fc&taille=40",
    "/api/v1/folders/4/messages?curseur=suite",
  ]);
});

it("garde par vue les pages chargées et les relit toutes avant de les remplacer", async () => {
  const [f6, f5, f4, f3] = [6, 5, 4, 3].map((n) => fil(`f${n}`, n * 100));
  const pages: Record<string, PageFils> = {
    "/api/v1/views/reception": page([f6, f5], "apres-f5"),
    "/api/v1/views/reception?curseur=apres-f5": page([f4, f3]),
    "/api/v1/views/flux": { ...page([f3], "apres-f3"), vue: "flux" },
  };
  const requete = vi.fn(async (url: string) => Response.json(pages[url]));
  vi.stubGlobal("fetch", requete);
  const courrier = creerCourrier(3);
  const reception = courrier.obtenirVue("reception");
  const flux = courrier.obtenirVue("flux");
  await Promise.all([reception.charger(), flux.charger()]);
  await reception.suite.charger();
  expect(ids(fenetre(reception)).nouveaux).toEqual(["f6", "f5", "f4", "f3"]);
  expect(fenetre(reception).curseur_suivant).toBeNull();
  expect(ids(fenetre(flux)).nouveaux).toEqual(["f3"]);
  expect(fenetre(flux).curseur_suivant).toBe("apres-f3");

  // Le fil f4 est lu depuis un autre appareil : la relecture couvre sa page.
  const relu = { ...f4, non_lu: false };
  pages["/api/v1/views/reception?curseur=apres-f5&taille=2"] = page([relu, f3]);
  requete.mockClear();
  await courrier.chargerVue("reception");
  expect(requete.mock.calls.map(([url]) => url)).toEqual([
    "/api/v1/views/reception",
    "/api/v1/views/reception?curseur=apres-f5&taille=2",
  ]);
  expect(ids(fenetre(reception))).toEqual({
    nouveaux: ["f6", "f5", "f3"],
    deja_vus: ["f4"],
  });
  flush();
  expect(reception.etat()).toMatchObject({
    statut: "pret",
    valeur: { deja_vus: [relu] },
    perime: false,
  });
});

it("conserve la page arrivée pendant qu’une relecture voyage encore", async () => {
  const [f6, f5, f4, f3] = [6, 5, 4, 3].map((n) => fil(`f${n}`, n * 100));
  const haut = page([f6, f5], "apres-f5");
  const requete = vi
    .fn<typeof fetch>()
    .mockResolvedValueOnce(Response.json(haut));
  vi.stubGlobal("fetch", requete);
  const courrier = creerCourrier(3);
  const entree = courrier.obtenirVue("reception");
  expect(entree.source.valeur()).toBeUndefined();
  await entree.charger();
  const relecture = reponseRetenue();
  requete
    .mockReturnValueOnce(relecture.promise)
    .mockResolvedValueOnce(Response.json(page([f4, f3], "apres-f3")));
  const actualisation = entree.charger();
  await entree.suite.charger();
  expect(ids(fenetre(entree)).nouveaux).toEqual(["f6", "f5", "f4", "f3"]);
  const etoile = { ...f6, etoile: true };
  relecture.resolve(Response.json(page([etoile, f5], "apres-f5")));
  await actualisation;
  expect(fenetre(entree).nouveaux).toEqual([etoile, f5, f4, f3]);
  expect(fenetre(entree).curseur_suivant).toBe("apres-f3");
  expect(requete).toHaveBeenCalledTimes(3);
});
