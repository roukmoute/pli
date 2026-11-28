import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import { reponseRetenue } from "../../tests/reponse-retenue";
import { creerSelection } from "./selection";

function choisis(choix: ReturnType<typeof creerSelection>) {
  flush();
  return [...choix.choisis()];
}

function echecs(choix: ReturnType<typeof creerSelection>) {
  flush();
  return choix.echecs();
}

it("retire le lot de la sélection dès le geste et y ramène les fils refusés", async () => {
  const retenue = reponseRetenue<string[]>();
  const lancer = vi.fn(() => retenue.promise);
  const choix = creerSelection(lancer);
  expect(echecs(choix)).toBe(0);
  choix.cocher(["a", "b", "c", "d"], true);
  choix.cocher(["d"], false);
  expect(choisis(choix)).toEqual(["a", "b", "c"]);
  await choix.marquer([], "lu", false);
  expect(lancer).not.toHaveBeenCalled();
  // Le fil d fait partie du lot sans être coché : un refus ne le sélectionne pas.
  const ecriture = choix.marquer(["a", "b", "d"], "lu", true);
  expect(lancer.mock.calls).toEqual([["lu", ["a", "b", "d"]]]);
  expect(choisis(choix)).toEqual(["c"]);
  expect(echecs(choix)).toBe(0);
  retenue.resolve(["b", "d"]);
  await ecriture;
  expect(choisis(choix)).toEqual(["c", "b"]);
  expect(echecs(choix)).toBe(2);
});

it("additionne les refus des actions lancées depuis le dernier geste", async () => {
  const premiere = reponseRetenue<string[]>();
  const seconde = reponseRetenue<string[]>();
  const lancer = vi
    .fn()
    .mockReturnValueOnce(Promise.resolve(["a"]))
    .mockReturnValueOnce(premiere.promise)
    .mockReturnValueOnce(seconde.promise);
  const choix = creerSelection(lancer);
  choix.cocher(["a", "b"], true);
  flush();
  await choix.marquer(["a"], "etoile", false);
  expect(lancer.mock.calls).toEqual([["sans_etoile", ["a"]]]);
  // L’étoile ne touche pas à la sélection.
  expect(choisis(choix)).toEqual(["a", "b"]);
  expect(echecs(choix)).toBe(1);
  // Un nouveau geste efface l’annonce précédente.
  const lue = choix.marquer(["a", "b"], "lu", false);
  expect(echecs(choix)).toBe(0);
  const etoilee = choix.marquer(["b"], "etoile", true);
  expect(lancer.mock.calls.slice(1)).toEqual([
    ["non_lu", ["a", "b"]],
    ["etoile", ["b"]],
  ]);
  premiere.resolve(["a", "b"]);
  await lue;
  expect(echecs(choix)).toBe(2);
  seconde.resolve(["b"]);
  await etoilee;
  expect(echecs(choix)).toBe(3);
  expect(choisis(choix)).toEqual(["a", "b"]);
});

it("retire les fils archivés de la sélection et n’y ramène que les fils refusés", async () => {
  const retenue = reponseRetenue<string[]>();
  const lancer = vi.fn(
    (_type: string, _ids: string[], _dossier?: number) => retenue.promise,
  );
  const choix = creerSelection(lancer);
  choix.cocher(["a", "b", "c"], true);
  flush();
  await choix.partir("archiver", []);
  expect(lancer).not.toHaveBeenCalled();
  const ecriture = choix.partir("archiver", ["a", "b", "d"]);
  expect(lancer.mock.calls).toEqual([["archiver", ["a", "b", "d"], undefined]]);
  expect(choisis(choix)).toEqual(["c"]);
  expect(echecs(choix)).toBe(0);
  retenue.resolve(["b", "d"]);
  await ecriture;
  // Le fil d n’était pas coché : refusé, il est proposé à la reprise comme b.
  expect(choisis(choix)).toEqual(["c", "b", "d"]);
  expect(echecs(choix)).toBe(2);
});

it("transmet le dossier choisi pour une restauration sans origine", async () => {
  const lancer = vi.fn(
    async (_type: string, _ids: string[], _dossier?: number) => ["b"],
  );
  const choix = creerSelection(lancer);
  await choix.partir("restaurer", ["a", "b"], 7);
  expect(lancer.mock.calls).toEqual([["restaurer", ["a", "b"], 7]]);
  expect(choisis(choix)).toEqual(["b"]);
  expect(echecs(choix)).toBe(1);
});

it("annonce un vidage que la messagerie n’a pas entièrement confirmé", async () => {
  const lancer = vi
    .fn<(type: string, ids: string[]) => Promise<string[]>>()
    .mockResolvedValue([])
    .mockResolvedValueOnce(["a"])
    .mockResolvedValueOnce(["dossier/8"]);
  const choix = creerSelection(lancer);
  const refus = () => {
    flush();
    return choix.vidageRefuse();
  };
  expect(refus()).toBe(false);
  await choix.partir("corbeille", ["a"]);
  expect(echecs(choix)).toBe(1);
  // Un vidage efface l’annonce précédente, puis signale son propre refus.
  await choix.vider("dossier/8");
  expect(lancer.mock.calls[1]).toEqual(["vider_dossier", ["dossier/8"]]);
  expect(echecs(choix)).toBe(0);
  expect(refus()).toBe(true);
  await choix.vider("dossier/8");
  expect(refus()).toBe(false);
  await choix.vider("dossier/8");
  await choix.partir("corbeille", ["a"]);
  expect(refus()).toBe(false);
});
