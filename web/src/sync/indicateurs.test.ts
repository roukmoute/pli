import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import { projeterAttentes } from "./effets";
import {
  conclureIndicateur,
  projeterIndicateur,
  typeOperation,
} from "./indicateurs";
import type { EnAttente, TypeOperation } from "./operations";
import type { PageFils } from "./vues";

// Trois fils du plus récent au plus ancien : a et c non lus, b lu.
const a = fil("a", 30);
const b = fil("b", 20, false);
const c = fil("c", 10);
const reception = page([a, b, c]);

function operation(type: TypeOperation, ...cibles: string[]): EnAttente {
  return { id: "operation", type, vue: "reception", cibles };
}

function trouver(lot: PageFils, id: string) {
  return [...lot.nouveaux, ...lot.deja_vus].find((f) => f.id === id);
}

it("traduit un indicateur et sa valeur en type d’opération", () => {
  expect(typeOperation("etoile", true)).toBe("etoile");
  expect(typeOperation("etoile", false)).toBe("sans_etoile");
  expect(typeOperation("lu", true)).toBe("lu");
  expect(typeOperation("lu", false)).toBe("non_lu");
});

it("pose ou retire l’étoile sans déplacer ni recréer les autres fils", () => {
  const etoilee = projeterIndicateur(
    reception,
    "reception",
    operation("etoile", "b", "absent"),
  );
  expect(ids(etoilee)).toEqual(ids(reception));
  expect(trouver(etoilee, "b")).toEqual({ ...b, etoile: true });
  expect(trouver(etoilee, "a")).toBe(a);
  expect(trouver(etoilee, "c")).toBe(c);
  const retiree = projeterIndicateur(
    etoilee,
    "reception",
    operation("sans_etoile", "b"),
  );
  expect(trouver(retiree, "b")).toEqual(b);
});

it("range un fil lu ou non lu dans sa section à sa place chronologique", () => {
  const lue = projeterIndicateur(
    reception,
    "reception",
    operation("lu", "a", "c"),
  );
  expect(ids(lue)).toEqual({ nouveaux: [], deja_vus: ["a", "b", "c"] });
  expect(trouver(lue, "a")).toEqual({ ...a, non_lu: false });
  const nonLue = projeterIndicateur(
    lue,
    "reception",
    operation("non_lu", "b", "c"),
  );
  expect(ids(nonLue)).toEqual({ nouveaux: ["b", "c"], deja_vus: ["a"] });
  expect(trouver(nonLue, "c")).toEqual(c);
});

it("laisse intacte la page d’une autre vue ou sans aucune cible", () => {
  const ailleurs = projeterIndicateur(reception, "flux", operation("lu", "a"));
  expect(ailleurs).toBe(reception);
  const sansCible = projeterIndicateur(
    reception,
    "reception",
    operation("lu", "absent"),
  );
  expect(sansCible).toBe(reception);
});

it("signale chaque action en attente sur le fil qu’elle touche", () => {
  const pret = { statut: "pret", valeur: reception, perime: true } as const;
  expect(projeterAttentes(pret, "reception", [])).toBe(pret);
  const projete = projeterAttentes(pret, "reception", [
    operation("lu", "a"),
    operation("etoile", "a", "b"),
    operation("non_lu", "a"),
  ]);
  expect(projete.statut === "pret" && projete.perime).toBe(true);
  const affichee = projete.statut === "pret" ? projete.valeur : reception;
  // Lu puis non lu : la dernière intention l’emporte.
  expect(ids(affichee)).toEqual(ids(reception));
  expect(trouver(affichee, "a")).toEqual({
    ...a,
    etoile: true,
    attente: { lu: true, etoile: true },
  });
  expect(trouver(affichee, "b")).toEqual({
    ...b,
    etoile: true,
    attente: { etoile: true },
  });
  expect(trouver(affichee, "c")).toBe(c);
  // Le cache, lui, ne garde aucun signe d’attente.
  expect(trouver(reception, "a")).toBe(a);
  for (const statut of ["chargement", "erreur"] as const) {
    const etat = { statut };
    expect(projeterAttentes(etat, "reception", [operation("lu", "a")])).toBe(
      etat,
    );
  }
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

it("inscrit les cibles confirmées dans le cache et relit les vues qui les montrent", async () => {
  const vues = {
    reception: cache(reception),
    "dossier/8": cache(page([b])),
    flux: cache(page([c])),
    justificatifs: cache(),
  };
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const etoiler = operation("etoile", "a", "b");
  await conclureIndicateur(Object.entries(vues), lectures, etoiler, ["a"], 3);
  const corrigee = vues.reception.source.courante as PageFils;
  expect(trouver(corrigee, "a")).toEqual({ ...a, etoile: true });
  expect(trouver(corrigee, "b")).toBe(b);
  expect(vues["dossier/8"].source.corriger).not.toHaveBeenCalled();
  expect(vues["dossier/8"].source.courante).toEqual(page([b]));
  const relues = Object.entries(vues)
    .filter(([, { source }]) => source.recharger.mock.calls.length)
    .map(([vue]) => vue);
  expect(relues).toEqual(["reception", "dossier/8"]);
  expect(vues.reception.source.recharger).toHaveBeenCalledTimes(1);
  expect(lectures.actualiserFils).not.toHaveBeenCalled();
});

it("ne réécrit pas une page relue depuis l’écriture et oublie les fils dont la lecture change", async () => {
  for (const type of ["lu", "non_lu"] as const) {
    for (const revision of [2, 1]) {
      // La page servie porte la révision 2.
      const vues = { reception: cache(reception) };
      const lectures = { actualiserFils: vi.fn(async () => {}) };
      const marquer = operation(type, "a", "b");
      await conclureIndicateur(
        Object.entries(vues),
        lectures,
        marquer,
        ["a", "b"],
        revision,
      );
      expect(vues.reception.source.courante).toBe(reception);
      expect(vues.reception.source.recharger).toHaveBeenCalledTimes(1);
      expect(lectures.actualiserFils.mock.calls).toEqual([[["a", "b"]]]);
    }
  }
});
