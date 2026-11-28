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
