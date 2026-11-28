import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import {
  conclureCopie,
  conclureDepart,
  conclureVidage,
  laisser,
  partances,
  retirerFils,
  viderVue,
} from "./departs";
import type { EnAttente, TypeOperation } from "./operations";
import type { PageFils } from "./vues";

// Du plus récent au plus ancien : a et c non lus, b et d lus.
const a = fil("a", 40);
const b = fil("b", 30, false);
const c = fil("c", 20);
const d = fil("d", 10, false);
const reception = page([a, b, c, d]);

function operation(
  type: TypeOperation,
  vue: string,
  ...cibles: string[]
): EnAttente {
  return { id: "operation", type, vue, cibles };
}

it("retire les fils archivés de la seule vue où le geste a eu lieu", () => {
  const archiver = operation("archiver", "reception", "a", "b", "absent");
  const allegee = retirerFils(reception, "reception", archiver);
  expect(ids(allegee)).toEqual({ nouveaux: ["c"], deja_vus: ["d"] });
  // Les fils restants ne sont pas recréés.
  expect(allegee.nouveaux[0]).toBe(c);
  expect(allegee.deja_vus[0]).toBe(d);
  expect(allegee.revision).toBe(reception.revision);
  // Sans total servi, rien n’est inventé. Avec lui, il suit les départs.
  expect(allegee.total).toBeUndefined();
  const comptee = { ...reception, total: 9 };
  expect(retirerFils(comptee, "reception", archiver).total).toBe(7);
  expect(retirerFils(reception, "flux", archiver)).toBe(reception);
  const ailleurs = operation("archiver", "reception", "absent");
  expect(retirerFils(reception, "reception", ailleurs)).toBe(reception);
});

it("compte par type les départs en attente de la vue", () => {
  const attentes = [
    operation("archiver", "reception", "a", "b"),
    operation("lu", "reception", "c"),
    operation("archiver", "flux", "d"),
    operation("corbeille", "reception", "e"),
    operation("archiver", "reception", "f"),
    operation("vider_dossier", "dossier/8", "dossier/8"),
    operation("copier", "reception", "g", "h"),
    operation("deplacer", "reception", "i"),
  ];
  expect(partances(attentes, "reception")).toEqual([
    { type: "archiver", nombre: 3 },
    { type: "corbeille", nombre: 1 },
    { type: "deplacer", nombre: 1 },
    { type: "copier", nombre: 2 },
  ]);
  expect(partances(attentes, "flux")).toEqual([
    { type: "archiver", nombre: 1 },
  ]);
  expect(partances(attentes, "dossier/8")).toEqual([
    { type: "vider_dossier", nombre: 1 },
  ]);
  expect(partances(attentes, "justificatifs")).toEqual([]);
});

it("vide la liste du seul dossier visé", () => {
  const corbeille = { ...page([a, b]), curseur_suivant: "suite" };
  const vider = operation("vider_dossier", "dossier/8", "dossier/8");
  expect(viderVue(corbeille, "dossier/8", vider)).toEqual({
    ...corbeille,
    nouveaux: [],
    deja_vus: [],
    curseur_suivant: null,
    total: 0,
  });
  expect(viderVue(corbeille, "dossier/9", vider)).toBe(corbeille);
  // Un vidage refusé ne désigne plus aucune cible : la liste reste entière.
  const refuse = operation("vider_dossier", "dossier/8");
  expect(viderVue(corbeille, "dossier/8", refuse)).toBe(corbeille);
});

function cache(valeur?: PageFils) {
  const source = {
    courante: valeur,
    valeur: () => source.courante,
    corriger: vi.fn((maj: (connue: PageFils) => PageFils) => {
      if (source.courante) source.courante = maj(source.courante);
    }),
    recharger: vi.fn(async () => {}),
  };
  return { source };
}

function relues(vues: Record<string, ReturnType<typeof cache>>) {
  return Object.entries(vues)
    .filter(([, { source }]) => source.recharger.mock.calls.length)
    .map(([vue]) => vue);
}

it("retire du cache les fils confirmés puis relit les vues touchées et les dossiers chargés", async () => {
  const vues = {
    reception: cache(reception),
    // Le dossier chargé peut recevoir les lettres. Celui jamais ouvert attendra sa visite.
    "dossier/8": cache(page([d])),
    "dossier/9": cache(),
    flux: cache(page([b])),
    justificatifs: cache(page([c])),
  };
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const archiver = operation("archiver", "reception", "a", "b");
  await conclureDepart(Object.entries(vues), lectures, archiver, ["a"], 3);
  // Le fil b, refusé, reste en cache : la liste le montre de nouveau.
  expect(ids(vues.reception.source.courante as PageFils)).toEqual({
    nouveaux: ["c"],
    deja_vus: ["b", "d"],
  });
  for (const vue of [
    "dossier/8",
    "dossier/9",
    "flux",
    "justificatifs",
  ] as const)
    expect(vues[vue].source.corriger).not.toHaveBeenCalled();
  expect(relues(vues)).toEqual(["reception", "dossier/8", "flux"]);
  expect(vues.reception.source.recharger).toHaveBeenCalledTimes(1);
  expect(lectures.actualiserFils.mock.calls).toEqual([[["a", "b"]]]);
});

it("ne réécrit pas une page relue depuis l’écriture et relit la vue du geste même sans cible", async () => {
  // La page servie porte la révision 2 et ne montre déjà plus le fil archivé.
  const sansCible = page([c, d]);
  const vues = { reception: cache(sansCible), flux: cache(page([a])) };
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const archiver = operation("archiver", "flux", "a");
  await conclureDepart(Object.entries(vues), lectures, archiver, ["a"], 2);
  expect(vues.flux.source.courante).toEqual(page([a]));
  expect(relues(vues)).toEqual(["flux"]);
  const partie = operation("archiver", "reception", "a");
  await conclureDepart(Object.entries(vues), lectures, partie, ["a"], 3);
  expect(vues.reception.source.courante).toBe(sansCible);
  expect(vues.reception.source.recharger).toHaveBeenCalledTimes(1);
});

it("vide le cache du dossier quand la messagerie a confirmé son vidage", async () => {
  const vues = {
    "dossier/8": cache(page([a, b])),
    reception: cache(reception),
  };
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const vider = operation("vider_dossier", "dossier/8", "dossier/8");
  await conclureVidage(Object.entries(vues), lectures, vider, [], 3);
  expect(ids(vues["dossier/8"].source.courante as PageFils)).toEqual({
    nouveaux: ["a"],
    deja_vus: ["b"],
  });
  await conclureVidage(Object.entries(vues), lectures, vider, ["dossier/8"], 3);
  expect(ids(vues["dossier/8"].source.courante as PageFils)).toEqual({
    nouveaux: [],
    deja_vus: [],
  });
  expect(vues.reception.source.courante).toBe(reception);
  expect(relues(vues)).toEqual(["dossier/8"]);
});

it("laisse la liste de départ d’une copie telle quelle et relit les dossiers chargés", async () => {
  expect(laisser(reception)).toBe(reception);
  const vues = { reception: cache(reception), "dossier/8": cache(page([])) };
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const copier = operation("copier", "reception", "a");
  await conclureCopie(Object.entries(vues), lectures, copier, ["a"], 3);
  expect(vues.reception.source.courante).toBe(reception);
  expect(relues(vues)).toEqual(["reception", "dossier/8"]);
});
