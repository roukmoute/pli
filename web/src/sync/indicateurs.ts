import type { Lectures } from "./lecture";
import type { EnAttente, Intention, TypeOperation } from "./operations";
import { comparerFils } from "./projection-tri";
import type { ressource } from "./ressource";
import type { Fil, PageFils } from "./vues";

export type Indicateur = "lu" | "etoile";

export function typeOperation(
  indicateur: Indicateur,
  valeur: boolean,
): TypeOperation {
  if (indicateur === "etoile") return valeur ? "etoile" : "sans_etoile";
  return valeur ? "lu" : "non_lu";
}

function appliquer(fil: Fil, type: TypeOperation, attente: boolean): Fil {
  const etoile = type === "etoile" || type === "sans_etoile";
  const suite: Fil = etoile
    ? { ...fil, etoile: type === "etoile" }
    : { ...fil, non_lu: type === "non_lu" };
  if (attente)
    suite.attente = { ...fil.attente, [etoile ? "etoile" : "lu"]: true };
  return suite;
}

/**
 * Montre l’effet d’une action dans la vue où elle a été demandée.
 * Avec `attente`, les fils touchés signalent que la messagerie n’a pas encore confirmé.
 */
export function projeterIndicateur(
  page: PageFils,
  vue: string,
  operation: Intention,
  attente = false,
): PageFils {
  const cibles = new Set(operation.cibles);
  const touche = (fil: Fil) => cibles.has(fil.id);
  if (
    operation.vue !== vue ||
    ![...page.nouveaux, ...page.deja_vus].some(touche)
  )
    return page;
  const projeter = (fils: Fil[]) =>
    fils.map((fil) =>
      touche(fil) ? appliquer(fil, operation.type, attente) : fil,
    );
  const fils = [...projeter(page.nouveaux), ...projeter(page.deja_vus)];
  return {
    ...page,
    nouveaux: fils.filter((fil) => fil.non_lu).sort(comparerFils),
    deja_vus: fils.filter((fil) => !fil.non_lu).sort(comparerFils),
  };
}

interface Cache {
  source: Pick<
    ReturnType<typeof ressource<PageFils>>,
    "valeur" | "corriger" | "recharger"
  >;
}

/**
 * Inscrit dans le cache l’effet des cibles confirmées, puis relit les vues concernées.
 * Une cible refusée retrouve ainsi l’état rétabli par le serveur.
 */
export function conclureIndicateur(
  vues: Iterable<[string, Cache]>,
  lectures: Pick<Lectures, "actualiserFils">,
  operation: EnAttente,
  confirmees: string[],
  revision: number,
): Promise<unknown> {
  const cibles = new Set(operation.cibles);
  const relectures: Promise<void>[] = [];
  if (operation.type === "lu" || operation.type === "non_lu")
    relectures.push(lectures.actualiserFils(operation.cibles));
  for (const [vue, { source }] of vues) {
    const page = source.valeur();
    const presente =
      page &&
      [...page.nouveaux, ...page.deja_vus].some((f) => cibles.has(f.id));
    if (vue === operation.vue)
      // Une page déjà relue après l’écriture contient son effet.
      source.corriger((connue) =>
        connue.revision < revision
          ? projeterIndicateur(connue, vue, {
              ...operation,
              cibles: confirmees,
            })
          : connue,
      );
    if (vue === operation.vue || presente) relectures.push(source.recharger());
  }
  return Promise.all(relectures);
}
