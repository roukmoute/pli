import { createSignal } from "solid-js";
import { type Indicateur, typeOperation } from "./indicateurs";
import type { TypeOperation } from "./operations";

/** Lance l’action sur ces fils et rend ceux que la messagerie n’a pas confirmés. */
type Lancer = (type: TypeOperation, ids: string[]) => Promise<string[]>;

export function creerSelection(lancer: Lancer) {
  const [choisis, choisir] = createSignal<ReadonlySet<string>>(new Set());
  const [echecs, signaler] = createSignal(0);

  function cocher(ids: string[], valeur: boolean) {
    choisir((avant) => {
      const apres = new Set(avant);
      for (const id of ids) {
        if (valeur) apres.add(id);
        else apres.delete(id);
      }
      return apres;
    });
  }

  // Le lot quitte la sélection dès le geste. Les fils refusés y reviennent pour réessayer.
  async function executer(type: TypeOperation, ids: string[], lot: string[]) {
    if (!ids.length) return;
    signaler(0);
    cocher(lot, false);
    const refuses = new Set(await lancer(type, ids));
    cocher(
      lot.filter((id) => refuses.has(id)),
      true,
    );
    signaler((nombre) => nombre + refuses.size);
  }

  function marquer(ids: string[], indicateur: Indicateur, valeur: boolean) {
    // L’étoile ne touche pas à la sélection.
    const lot =
      indicateur === "lu" ? ids.filter((id) => choisis().has(id)) : [];
    return executer(typeOperation(indicateur, valeur), ids, lot);
  }

  /** Les fils archivés partent avec leur coche. Seuls les fils refusés sont proposés à la reprise. */
  function archiver(ids: string[]) {
    return executer("archiver", ids, ids);
  }
  return { choisis, cocher, marquer, archiver, echecs };
}
export type Selection = ReturnType<typeof creerSelection>;
