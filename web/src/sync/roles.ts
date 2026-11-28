import type { Etat } from "./ressource";
import type { Dossier } from "./vues";

/** Dossier que cette vue affiche. Une vue logique, comme Réception, n’en désigne aucun. */
export function dossierDeVue(catalogue: Etat<Dossier[]>, vue: string) {
  if (catalogue.statut !== "pret") return;
  return catalogue.valeur.find((d) => `dossier/${d.id}` === vue);
}

/** Dossiers où remettre du courrier restauré : ceux qui en reçoivent, hors Corbeille et Indésirables. */
export function dossiersDeRestauration(catalogue: Etat<Dossier[]>) {
  if (catalogue.statut !== "pret") return [];
  return catalogue.valeur.filter(
    (d) => d.selectable && d.role !== "trash" && d.role !== "junk",
  );
}
