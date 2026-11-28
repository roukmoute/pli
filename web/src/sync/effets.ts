import {
  conclureCopie,
  conclureDepart,
  conclureVidage,
  laisser,
  retirerFils,
  viderVue,
} from "./departs";
import { conclureIndicateur, projeterIndicateur } from "./indicateurs";
import type { EnAttente, TypeOperation } from "./operations";
import type { Etat } from "./ressource";
import type { PageFils } from "./vues";

// Chaque type d’opération montre son effet avant la réponse, puis conclut.
const drapeau = { projeter: projeterIndicateur, conclure: conclureIndicateur };
const depart: typeof drapeau = {
  projeter: retirerFils,
  conclure: conclureDepart,
};
const vidage: typeof drapeau = { projeter: viderVue, conclure: conclureVidage };
const copie: typeof drapeau = { projeter: laisser, conclure: conclureCopie };
const effets: Record<TypeOperation, typeof drapeau> = {
  etoile: drapeau,
  sans_etoile: drapeau,
  lu: drapeau,
  non_lu: drapeau,
  archiver: depart,
  corbeille: depart,
  restaurer: depart,
  supprimer_definitivement: depart,
  vider_dossier: vidage,
  deplacer: depart,
  copier: copie,
};

/** Superpose à la page servie les actions que la messagerie n’a pas encore confirmées. */
export function projeterAttentes(
  etat: Etat<PageFils>,
  vue: string,
  attentes: readonly EnAttente[],
): Etat<PageFils> {
  if (etat.statut !== "pret" || !attentes.length) return etat;
  return {
    ...etat,
    valeur: attentes.reduce(
      (page, operation) =>
        effets[operation.type].projeter(page, vue, operation, true),
      etat.valeur,
    ),
  };
}

/**
 * Inscrit dans le cache l’effet des cibles confirmées et rend la promesse des relectures.
 * Une cible refusée retrouve ainsi l’état rétabli par le serveur.
 */
export const conclureOperation: typeof conclureIndicateur = (
  vues,
  lectures,
  operation,
  ...suite
) => effets[operation.type].conclure(vues, lectures, operation, ...suite);
