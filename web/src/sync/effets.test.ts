import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import { conclureOperation, projeterAttentes } from "./effets";
import type { EnAttente, TypeOperation } from "./operations";
import type { PageFils } from "./vues";

const a = fil("a", 30);
const b = fil("b", 20, false);
const c = fil("c", 10);
const reception = page([a, b, c]);

function operation(type: TypeOperation, ...cibles: string[]): EnAttente {
  return { id: "operation", type, vue: "reception", cibles };
}

it("superpose chaque action en attente selon son type", () => {
  const pret = { statut: "pret", valeur: reception, perime: true } as const;
  const projete = projeterAttentes(pret, "reception", [
    operation("etoile", "a", "b"),
    operation("archiver", "a"),
    operation("lu", "c"),
  ]);
  expect(projete.statut === "pret" && projete.perime).toBe(true);
  const affichee = projete.statut === "pret" ? projete.valeur : reception;
  // Le fil archivé a quitté la liste. Les deux autres portent leur signe d’attente.
  expect(ids(affichee)).toEqual({ nouveaux: [], deja_vus: ["b", "c"] });
  expect(affichee.deja_vus).toEqual([
    { ...b, etoile: true, attente: { etoile: true } },
    { ...c, non_lu: false, attente: { lu: true } },
  ]);
  // Le cache garde tous ses fils.
  expect(ids(reception)).toEqual({ nouveaux: ["a", "c"], deja_vus: ["b"] });
});

it("conclut chaque opération selon son type", async () => {
  const lectures = { actualiserFils: vi.fn(async () => {}) };
  const conclure = async (type: TypeOperation) => {
    const source = {
      courante: reception,
      valeur: () => source.courante,
      corriger(maj: (connue: PageFils) => PageFils) {
        source.courante = maj(source.courante);
      },
      recharger: vi.fn(async () => {}),
    };
    const vues: [string, { source: typeof source }][] = [
      ["reception", { source }],
    ];
    await conclureOperation(
      vues,
      lectures,
      operation(type, "a", "b"),
      ["a"],
      3,
    );
    expect(source.recharger).toHaveBeenCalledTimes(1);
    return source.courante;
  };
  expect(ids(await conclure("archiver"))).toEqual({
    nouveaux: ["c"],
    deja_vus: ["b"],
  });
  const etoilee = await conclure("etoile");
  expect(etoilee.nouveaux[0]).toEqual({ ...a, etoile: true });
  expect(ids(await conclure("lu"))).toEqual({
    nouveaux: ["c"],
    deja_vus: ["a", "b"],
  });
  expect(ids(await conclure("non_lu"))).toEqual(ids(reception));
  expect((await conclure("sans_etoile")).nouveaux[0]).toEqual(a);
  // Jeter, restaurer ou effacer retire aussi les seuls fils confirmés.
  for (const type of [
    "corbeille",
    "restaurer",
    "supprimer_definitivement",
  ] as const)
    expect(ids(await conclure(type))).toEqual({
      nouveaux: ["c"],
      deja_vus: ["b"],
    });
  expect(ids(await conclure("vider_dossier"))).toEqual({
    nouveaux: [],
    deja_vus: [],
  });
});

it("vide la liste dès la demande de vidage", () => {
  const pret = { statut: "pret", valeur: reception, perime: false } as const;
  const projete = projeterAttentes(pret, "reception", [
    operation("vider_dossier", "reception"),
  ]);
  const affichee = projete.statut === "pret" ? projete.valeur : reception;
  expect(ids(affichee)).toEqual({ nouveaux: [], deja_vus: [] });
});
