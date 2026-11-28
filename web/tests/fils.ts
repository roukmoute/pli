import type { Fil, PageFils } from "../src/sync";
import modele from "./fil.json";
import receptionVide from "./reception-vide.json";

export function fil(id: string, date: number, nonLu = true): Fil {
  return { ...modele, id, objet: `Lettre ${id}`, date, non_lu: nonLu };
}

export function page(
  fils: Fil[],
  curseur: string | null = null,
  revision = 2,
): PageFils {
  return {
    ...receptionVide,
    nouveaux: fils.filter((f) => f.non_lu),
    deja_vus: fils.filter((f) => !f.non_lu),
    curseur_suivant: curseur,
    revision,
  };
}

export function ids(lot: PageFils) {
  return {
    nouveaux: lot.nouveaux.map((f) => f.id),
    deja_vus: lot.deja_vus.map((f) => f.id),
  };
}
