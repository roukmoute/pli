import { createSignal } from "solid-js";
import { type Indicateur, typeOperation } from "./indicateurs";
import type { TypeOperation } from "./operations";

/**
 * Lance l’action sur ces cibles et rend celles que la messagerie n’a pas confirmées.
 * Le dossier accompagne une restauration dont l’origine est inconnue.
 */
type Lancer = (
  type: TypeOperation,
  ids: string[],
  dossier?: number,
) => Promise<string[]>;

/** Actions qui font quitter leur liste aux fils. */
export type Depart =
  | "archiver"
  | "corbeille"
  | "restaurer"
  | "supprimer_definitivement";

export function creerSelection(lancer: Lancer) {
  const [choisis, choisir] = createSignal<ReadonlySet<string>>(new Set());
  const [echecs, signaler] = createSignal(0);
  const [vidageRefuse, refuserVidage] = createSignal(false);

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
  async function executer(lot: string[], envoyer: () => Promise<string[]>) {
    signaler(0);
    refuserVidage(false);
    cocher(lot, false);
    const refuses = new Set(await envoyer());
    cocher(
      lot.filter((id) => refuses.has(id)),
      true,
    );
    signaler((nombre) => nombre + refuses.size);
  }

  async function marquer(
    ids: string[],
    indicateur: Indicateur,
    valeur: boolean,
  ) {
    if (!ids.length) return;
    // L’étoile ne touche pas à la sélection.
    const lot =
      indicateur === "lu" ? ids.filter((id) => choisis().has(id)) : [];
    await executer(lot, () => lancer(typeOperation(indicateur, valeur), ids));
  }

  /** Les fils partent avec leur coche. Seuls les fils refusés sont proposés à la reprise. */
  async function partir(type: Depart, ids: string[], dossier?: number) {
    if (!ids.length) return;
    await executer(ids, () => lancer(type, ids, dossier));
  }

  /** Vide le dossier affiché. Un refus, même partiel, est annoncé. */
  async function vider(dossier: string) {
    signaler(0);
    refuserVidage(false);
    const refuses = await lancer("vider_dossier", [dossier]);
    refuserVidage(refuses.length > 0);
  }
  return { choisis, cocher, marquer, partir, vider, echecs, vidageRefuse };
}
export type Selection = ReturnType<typeof creerSelection>;
