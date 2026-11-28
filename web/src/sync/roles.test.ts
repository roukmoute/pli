import { expect, it } from "vitest";
import catalogue from "../../tests/dossiers.json";
import { dossierDeVue, dossiersDeRestauration } from "./roles";
import type { Dossier } from "./vues";

const { inbox, atelier, classement, envoyes, archives, corbeille, reclames } =
  catalogue as Record<string, Dossier>;
const tous = [
  inbox,
  atelier,
  classement,
  envoyes,
  archives,
  corbeille,
  reclames,
];
const pret = { statut: "pret", valeur: tous, perime: false } as const;

it("retrouve le dossier affiché par une vue", () => {
  expect(dossierDeVue(pret, "dossier/7")).toBe(archives);
  expect(dossierDeVue(pret, "dossier/1")).toBe(inbox);
  expect(dossierDeVue(pret, "dossier/70")).toBeUndefined();
  // Une vue logique ne désigne aucun dossier, un catalogue absent non plus.
  expect(dossierDeVue(pret, "reception")).toBeUndefined();
  for (const statut of ["chargement", "erreur"] as const)
    expect(dossierDeVue({ statut }, "dossier/7")).toBeUndefined();
});

it("propose pour une restauration les dossiers ouvrables hors Corbeille et Indésirables", () => {
  expect(dossiersDeRestauration(pret)).toEqual([
    inbox,
    atelier,
    envoyes,
    archives,
  ]);
  for (const statut of ["chargement", "erreur"] as const)
    expect(dossiersDeRestauration({ statut })).toEqual([]);
});
