import { filsDe } from "./fenetre";
import type { Lectures } from "./lecture";
import type { EnAttente, Intention, TypeOperation } from "./operations";
import type { ressource } from "./ressource";
import type { Fil, PageFils } from "./vues";

/** Retire de la vue où le geste a eu lieu les fils qui la quittent. */
export function retirerFils(
  page: PageFils,
  vue: string,
  operation: Intention,
): PageFils {
  const cibles = new Set(operation.cibles);
  const part = (fil: Fil) => cibles.has(fil.id);
  const partis = filsDe(page).filter(part).length;
  if (operation.vue !== vue || !partis) return page;
  return {
    ...page,
    nouveaux: page.nouveaux.filter((fil) => !part(fil)),
    deja_vus: page.deja_vus.filter((fil) => !part(fil)),
    // Le total de la liste suit, quand le serveur l’a donné.
    total: page.total && page.total - partis,
  };
}

/** Vide la liste du dossier que l’opération vise. Sans cible, rien n’est vidé. */
export function viderVue(
  page: PageFils,
  vue: string,
  operation: Intention,
): PageFils {
  if (operation.vue !== vue || !operation.cibles.length) return page;
  return {
    ...page,
    nouveaux: [],
    deja_vus: [],
    curseur_suivant: null,
    total: 0,
  };
}

/** Départs que la messagerie n’a pas fini de traiter depuis cette vue, par type. */
export interface Partance {
  type: TypeOperation;
  nombre: number;
}

const DEPARTS: readonly TypeOperation[] = [
  "archiver",
  "corbeille",
  "restaurer",
  "supprimer_definitivement",
  "vider_dossier",
];

export function partances(
  attentes: readonly EnAttente[],
  vue: string,
): Partance[] {
  return DEPARTS.map((type) => ({
    type,
    nombre: attentes
      .filter((operation) => operation.type === type)
      .filter((operation) => operation.vue === vue)
      .flatMap((operation) => operation.cibles).length,
  })).filter((partance) => partance.nombre);
}

interface Cache {
  source: Pick<
    ReturnType<typeof ressource<PageFils>>,
    "valeur" | "corriger" | "recharger"
  >;
}

/**
 * Retire du cache l’effet confirmé, puis relit les vues qui montraient une cible
 * et les dossiers chargés : l’un d’eux reçoit les lettres. Un fil refusé y revient.
 */
function conclure(projeter: typeof retirerFils) {
  return (
    vues: Iterable<[string, Cache]>,
    lectures: Pick<Lectures, "actualiserFils">,
    operation: EnAttente,
    confirmees: string[],
    revision: number,
  ): Promise<unknown> => {
    const cibles = new Set(operation.cibles);
    // Une lettre déplacée change d’identifiant : les fils gardés en mémoire sont périmés.
    const relectures = [lectures.actualiserFils(operation.cibles)];
    for (const [vue, { source }] of vues) {
      const page = source.valeur();
      const concernee =
        page &&
        (vue.startsWith("dossier/") ||
          filsDe(page).some((fil) => cibles.has(fil.id)));
      if (vue === operation.vue)
        // Une page déjà relue après l’écriture ne montre plus ces fils.
        source.corriger((connue) =>
          connue.revision < revision
            ? projeter(connue, vue, { ...operation, cibles: confirmees })
            : connue,
        );
      if (vue === operation.vue || concernee)
        relectures.push(source.recharger());
    }
    return Promise.all(relectures);
  };
}

export const conclureDepart = conclure(retirerFils);
export const conclureVidage = conclure(viderVue);
