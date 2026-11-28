import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import { conclureArchivage, filsEnPartance, retirerFils } from "./archivage";
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
  expect(retirerFils(reception, "flux", archiver)).toBe(reception);
  const ailleurs = operation("archiver", "reception", "absent");
  expect(retirerFils(reception, "reception", ailleurs)).toBe(reception);
});

it("compte les fils en partance de la vue", () => {
  const attentes = [
    operation("archiver", "reception", "a", "b"),
    operation("lu", "reception", "c"),
    operation("archiver", "flux", "d"),
    operation("archiver", "reception", "e"),
  ];
  expect(filsEnPartance(attentes, "reception")).toBe(3);
  expect(filsEnPartance(attentes, "flux")).toBe(1);
  expect(filsEnPartance(attentes, "justificatifs")).toBe(0);
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
  await conclureArchivage(Object.entries(vues), lectures, archiver, ["a"], 3);
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
  await conclureArchivage(Object.entries(vues), lectures, archiver, ["a"], 2);
  expect(vues.flux.source.courante).toEqual(page([a]));
  expect(relues(vues)).toEqual(["flux"]);
  const partie = operation("archiver", "reception", "a");
  await conclureArchivage(Object.entries(vues), lectures, partie, ["a"], 3);
  expect(vues.reception.source.courante).toBe(sansCible);
  expect(vues.reception.source.recharger).toHaveBeenCalledTimes(1);
});
