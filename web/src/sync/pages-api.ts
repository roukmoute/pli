import { getJson } from "./api";
import type { Dossier, PageFils, Vue } from "./vues";

function estDossier(vue: Vue): vue is `dossier/${number}` {
  return vue.startsWith("dossier/");
}

export function lirePage(compte: number, vue: Vue): Promise<PageFils> {
  const id = estDossier(vue) ? Number(vue.slice("dossier/".length)) : undefined;
  const chemin =
    id === undefined
      ? (`/views/${vue}` as const)
      : (`/folders/${id}/messages` as const);
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
