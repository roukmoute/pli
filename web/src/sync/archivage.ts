import { filsDe } from "./fenetre";
import type { Lectures } from "./lecture";
import type { EnAttente, Intention } from "./operations";
import type { ressource } from "./ressource";
import type { Fil, PageFils } from "./vues";

/** Retire de la vue où l’archivage a été demandé les fils qui partent. */
export function retirerFils(
  page: PageFils,
  vue: string,
  operation: Intention,
): PageFils {
  const cibles = new Set(operation.cibles);
  const part = (fil: Fil) => cibles.has(fil.id);
  if (operation.vue !== vue || !filsDe(page).some(part)) return page;
  return {
    ...page,
    nouveaux: page.nouveaux.filter((fil) => !part(fil)),
    deja_vus: page.deja_vus.filter((fil) => !part(fil)),
  };
}

/** Nombre de fils que la messagerie n’a pas encore fini d’archiver depuis cette vue. */
export function filsEnPartance(attentes: readonly EnAttente[], vue: string) {
  return attentes
    .filter((operation) => operation.type === "archiver")
    .filter((operation) => operation.vue === vue)
    .flatMap((operation) => operation.cibles).length;
}

interface Cache {
  source: Pick<
    ReturnType<typeof ressource<PageFils>>,
    "valeur" | "corriger" | "recharger"
  >;
}

/**
 * Retire du cache les fils confirmés, puis relit les vues qui montraient une cible
 * et les dossiers chargés : l’un d’eux reçoit les lettres. Un fil refusé y revient.
 */
export function conclureArchivage(
  vues: Iterable<[string, Cache]>,
  lectures: Pick<Lectures, "actualiserFils">,
  operation: EnAttente,
  confirmees: string[],
  revision: number,
): Promise<unknown> {
  const cibles = new Set(operation.cibles);
  // Une lettre archivée change d’identifiant : les fils gardés en mémoire sont périmés.
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
          ? retirerFils(connue, vue, { ...operation, cibles: confirmees })
          : connue,
      );
    if (vue === operation.vue || concernee) relectures.push(source.recharger());
  }
  return Promise.all(relectures);
}
