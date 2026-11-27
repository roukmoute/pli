import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import { lireDossiers, lirePage } from "./pages-api";
import { creerCourrier, type Vue } from "./vues";

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
  expect(entree.repere).toEqual({ defilement: 0, section: "nouveaux" });
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

it("actualise la vue de l’action et les autres pages qui contiennent une cible", async () => {
  const courrier = creerCourrier(5);
  const source = vi.fn(async (url: string) => {
    if (url.endsWith("/flags"))
      return Response.json([{ id: "b", etat: "confirme" }]);
    if (url.includes("/threads/")) return Response.json([]);
    if (url.includes("folders/"))
      return Response.json({
        ...premiere,
        vue: undefined,
        dossier: 8,
        deja_vus: [{ id: "b" }],
      });
    return Response.json({ ...premiere, vue: url.split("/").at(-1) });
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
  source.mockClear();
  await entree.selection.marquer(["b"], "lu", false);
  expect(source.mock.calls.map(([url]) => url).sort()).toEqual([
    "/api/v1/folders/8/messages",
    "/api/v1/messages/flags",
    "/api/v1/threads/b",
    "/api/v1/views/reception",
  ]);
  source.mockClear();
  await entree.selection.marquer(["b"], "etoile", true);
  expect(source.mock.calls.map(([url]) => url).sort()).toEqual([
    "/api/v1/folders/8/messages",
    "/api/v1/messages/flags",
    "/api/v1/views/reception",
  ]);
  fermer();
});
