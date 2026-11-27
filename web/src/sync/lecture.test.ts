import { expect, it, vi } from "vitest";
import reception from "../../tests/reception-vide.json";
import { reponseRetenue } from "../../tests/reponse-retenue";
import { documentMessage } from "../courrier/document-message";
import { appliquerLecture, creerLectures, imageIntegree } from "./lecture";
import { creerCourrier, type Fil } from "./vues";

it("résout uniquement les références connues du message et garde les sources distantes sous contrôle", () => {
  const html = [
    '<img src="inconnu">',
    '<img src="cid:pli-9">',
    '<img src="cid:pli-0">',
    '<img src="cid:pli-1">',
    '<img src="cid:pli-2">',
    '<img src="cid:pli-3">',
    '<img alt="vide">',
  ].join("");
  const contenu = {
    html,
    pieces: [],
    images: [
      { type: "integree", partie: 4 },
      { type: "distante", url: "https://exemple.test/photo" },
      { type: "distante", url: "ftp://exemple.test/fichier" },
      { type: "integree", partie: 8 },
    ],
  } as const;
  const copie = { ...contenu, pieces: [], images: [...contenu.images] };
  const images = new Map([[4, "data:image/png;base64,QQ=="]]);
  function sources(accord: boolean) {
    const document = new DOMParser().parseFromString(
      documentMessage(copie, accord, images),
      "text/html",
    );
    return [...document.images].map((i) => i.getAttribute("src"));
  }
  expect(sources(false)).toEqual([
    null,
    null,
    images.get(4),
    null,
    null,
    null,
    null,
  ]);
  expect(sources(true)).toEqual([
    null,
    null,
    images.get(4),
    "https://exemple.test/photo",
    null,
    null,
    null,
  ]);
});

function ligne(id: string, non_lu: boolean, date: number): Fil {
  return {
    id,
    non_lu,
    date,
    objet: id,
    expediteur: { nom: null, adresse: null },
    extrait: "",
    nombre_messages: 1,
    etoile: false,
    repondu: false,
    transfere: false,
    piece_jointe: false,
  };
}
it("répartit les fils avec la révision de chaque vue et respecte les opérations concurrentes", async () => {
  const base = {
    ...reception,
    nouveaux: [ligne("a", true, 9), ligne("b", true, 3)],
    deja_vus: [ligne("c", false, 8)],
  };
  const lecture = {
    fil: "a",
    revision: base.revision,
    non_lus: { reception: false },
  };
  expect(appliquerLecture(base, "flux", lecture)).toBe(base);
  expect(
    appliquerLecture(base, "reception", {
      ...lecture,
      revision: base.revision - 1,
    }),
  ).toBe(base);
  const lu = appliquerLecture(base, "reception", lecture);
  expect(lu.nouveaux.map((f) => f.id)).toEqual(["b"]);
  expect(lu.deja_vus.map((f) => [f.id, f.non_lu])).toEqual([
    ["a", false],
    ["c", false],
  ]);
  const attente = reponseRetenue();
  const fetch = vi.fn(async (url: string) => {
    if (url.endsWith("/views/reception")) return Response.json(base);
    if (url.endsWith("/threads/a"))
      return Response.json([
        { id: "1", non_lu: true },
        { id: "2", non_lu: true },
      ]);
    if (url.endsWith("/messages/1/open")) return attente.promise;
    if (url.endsWith("/messages/2/open"))
      return Response.json({ ...lecture, revision: 6 });
    throw Error(url);
  });
  vi.stubGlobal("fetch", fetch);
  const courrier = creerCourrier(3);
  const fils = courrier.lectures.fil("a");
  const receptionCache = courrier.obtenirVue("reception");
  courrier.obtenirVue("flux");
  await Promise.all([fils.charger(), receptionCache.charger()]);
  const premiere = courrier.lectures.ouvrir("1", "a");
  expect(courrier.lectures.ouvrir("1", "a")).toBe(premiere);
  await courrier.lectures.ouvrir("2", "a");
  expect(fils.etat()).toMatchObject({
    valeur: [
      { id: "1", non_lu: true },
      { id: "2", non_lu: false },
    ],
  });
  attente.resolve(
    Response.json({ ...lecture, revision: 5, non_lus: { reception: true } }),
  );
  await premiere;
  expect(fils.etat()).toMatchObject({
    valeur: [
      { id: "1", non_lu: false },
      { id: "2", non_lu: false },
    ],
  });
  expect(receptionCache.etat()).toMatchObject({
    valeur: { deja_vus: [{ id: "a" }, { id: "c" }] },
  });
  expect(fetch.mock.calls).toHaveLength(4);
});

it("applique une seule fois la même révision après une ouverture rejouée", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      Response.json({
        fil: "jardin",
        revision: 6,
        non_lus: { reception: false },
      }),
    ),
  );
  const confirmer = vi.fn();
  const cache = creerLectures(confirmer);
  await cache.ouvrir("5", "jardin");
  await cache.ouvrir("5", "jardin");
  expect(confirmer).toHaveBeenCalledTimes(1);
});
it("borne chaque cache à quarante entrées et conserve un fil récemment relu", () => {
  const cache = creerLectures(() => {});
  const premiere = cache.fil("0"),
    seconde = cache.fil("1");
  for (let i = 2; i < 40; i++) cache.fil(String(i));
  expect(cache.fil("0")).toBe(premiere);
  cache.fil("40");
  expect(cache.fil("1")).not.toBe(seconde);
  expect(cache.fil("0")).toBe(premiere);
});
it("distingue l’identité du fil ouvert et rejette les types MIME inconnus", async () => {
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({ fil: "autre", revision: 9, non_lus: {} }),
    )
    .mockResolvedValueOnce(
      new Response("pièce", { headers: { "content-type": "text/plain" } }),
    );
  vi.stubGlobal("fetch", fetch);
  const confirmer = vi.fn();
  await expect(creerLectures(confirmer).ouvrir("1", "courant")).rejects.toThrow(
    "Fil",
  );
  expect(confirmer).not.toHaveBeenCalled();
  await expect(imageIntegree("4-5-6", 7)).rejects.toThrow("image");
});
