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

  async function marquer(
    ids: string[],
    indicateur: Indicateur,
    valeur: boolean,
  ) {
    if (!ids.length) return;
    signaler(0);
    // Un lot marqué quitte la sélection dès le geste. Un refus l’y ramène pour réessayer.
    const lot =
      indicateur === "lu" ? ids.filter((id) => choisis().has(id)) : [];
    cocher(lot, false);
    const refuses = new Set(
      await lancer(typeOperation(indicateur, valeur), ids),
    );
    cocher(
      lot.filter((id) => refuses.has(id)),
      true,
    );
    signaler((nombre) => nombre + refuses.size);
  }
  return { choisis, cocher, marquer, echecs };
}
export type Selection = ReturnType<typeof creerSelection>;
