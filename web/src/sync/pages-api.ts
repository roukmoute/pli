import { getJson } from "./api";
import type { Dossier, PageFils, Vue } from "./vues";

function estDossier(vue: Vue): vue is `dossier/${number}` {
  return vue.startsWith("dossier/");
}

/** Portion de liste demandée. Sans taille, le serveur applique son réglage. */
export interface Tranche {
  curseur?: string;
  taille?: number;
}

export function lirePage(
  compte: number,
  vue: Vue,
  tranche: Tranche = {},
): Promise<PageFils> {
  const id = estDossier(vue) ? Number(vue.slice("dossier/".length)) : undefined;
  const liste =
    id === undefined
      ? (`/views/${vue}` as const)
      : (`/folders/${id}/messages` as const);
  const suite = new URLSearchParams();
  if (tranche.curseur) suite.set("curseur", tranche.curseur);
  if (tranche.taille) suite.set("taille", String(tranche.taille));
  const requete = suite.toString();
  const chemin: `/${string}` = requete ? `${liste}?${requete}` : liste;
  return getJson(chemin, (corps) => {
    const page = corps as PageFils | null;
    if (
      page?.compte !== compte ||
      (id === undefined ? page.vue !== vue : page.dossier !== id) ||
      typeof page.couverture?.import_en_cours !== "boolean" ||
      !Number.isInteger(page.expediteurs_en_attente) ||
      !Array.isArray(page.nouveaux) ||
      !Array.isArray(page.deja_vus)
    ) {
      throw new Error("La liste reçue ne correspond pas à la boîte ouverte.");
    }
    return page;
  });
}

export function lireDossiers(): Promise<Dossier[]> {
  return getJson("/folders", (corps) => {
    const liste = corps as { folders?: Dossier[] } | null;
    if (!Array.isArray(liste?.folders))
      throw new Error("Liste de dossiers illisible.");
    return liste.folders.filter((dossier) => !dossier.hidden);
  });
}
