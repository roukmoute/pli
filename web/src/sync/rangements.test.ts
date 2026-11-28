import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import catalogue from "../../tests/dossiers.json";
import { creerRangements } from "./rangements";
import type { Dossier } from "./vues";

const { inbox, atelier, archives } = catalogue as Record<string, Dossier>;

function ouvrir(refuses: string[] = []) {
  const etat = {
    catalogue: [inbox, atelier, archives] as Dossier[] | undefined,
  };
  const dossiers = {
    valeur: () => etat.catalogue,
    recharger: vi.fn(async () => {}),
  };
  const selection = {
    partir: vi.fn(async () => refuses),
    copier: vi.fn(async () => refuses),
  };
  const selectionDe = vi.fn(() => selection);
  const rangements = creerRangements(dossiers, selectionDe);
  const demande = () => {
    flush();
    return rangements.demande();
  };
  return { etat, dossiers, selection, selectionDe, rangements, demande };
}

it("demande un dossier puis range les fils dès le choix", async () => {
  const { dossiers, selection, selectionDe, rangements, demande } = ouvrir();
  expect(demande()).toBeUndefined();
  const apres = vi.fn();
  rangements.proposer("deplacer", "reception", ["a", "b"], apres);
  expect(demande()).toEqual({
    type: "deplacer",
    vue: "reception",
    cibles: ["a", "b"],
    disparu: false,
    apres,
  });
  rangements.renoncer();
  expect(demande()).toBeUndefined();

  rangements.proposer("deplacer", "reception", ["a", "b"], apres);
  const rangement = rangements.ranger("deplacer", "reception", ["a"], 2);
  // Le geste part aussitôt et referme la demande.
  expect(selection.partir.mock.calls).toEqual([["deplacer", ["a"], 2]]);
  expect(selectionDe.mock.calls).toEqual([["reception"]]);
  expect(demande()).toBeUndefined();
  await rangement;
  await rangements.ranger("copier", "dossier/7", ["b"], 2);
  expect(selection.copier.mock.calls).toEqual([[["b"], 2]]);
  expect(selection.partir).toHaveBeenCalledTimes(1);
  // Tout est confirmé : le catalogue n’a pas à être relu.
  expect(dossiers.recharger).not.toHaveBeenCalled();
  expect(demande()).toBeUndefined();
  expect(apres).not.toHaveBeenCalled();
});

it("redemande un dossier pour les fils refusés quand le dossier choisi a disparu", async () => {
  const { etat, dossiers, rangements, demande } = ouvrir(["b"]);
  // Le dossier existe toujours : le refus vient d’ailleurs.
  await rangements.ranger("deplacer", "reception", ["a", "b"], 2);
  expect(dossiers.recharger).toHaveBeenCalledTimes(1);
  expect(demande()).toBeUndefined();
  // Catalogue illisible : rien ne permet d’accuser le dossier.
  etat.catalogue = undefined;
  await rangements.ranger("deplacer", "reception", ["a", "b"], 2);
  expect(demande()).toBeUndefined();

  etat.catalogue = [inbox, archives];
  await rangements.ranger("copier", "reception", ["a", "b"], 2);
  const reprise = demande();
  expect(reprise).toMatchObject({
    type: "copier",
    vue: "reception",
    cibles: ["b"],
    disparu: true,
  });
  expect(reprise?.apres()).toBeUndefined();
  expect(dossiers.recharger).toHaveBeenCalledTimes(3);
});
