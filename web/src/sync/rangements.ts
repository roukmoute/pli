import { createSignal } from "solid-js";
import type { ressource } from "./ressource";
import type { Selection } from "./selection";
import type { Dossier } from "./vues";

export type Rangement = "deplacer" | "copier";

/** Fils à ranger, en attente du choix de leur dossier. */
export interface DemandeDeRangement {
  type: Rangement;
  vue: string;
  cibles: string[];
  /** Vrai quand le dossier choisi a disparu avant de recevoir ces fils. */
  disparu: boolean;
  /** Après un déplacement lancé : replace le focus ou quitte le fil ouvert. */
  apres: () => void;
}

type Catalogue = Pick<
  ReturnType<typeof ressource<Dossier[]>>,
  "valeur" | "recharger"
>;

/**
 * Déplace ou copie des fils vers un dossier choisi. Le menu et le glisser-déposer
 * passent tous deux par `ranger`.
 */
export function creerRangements(
  dossiers: Catalogue,
  selectionDe: (vue: string) => Pick<Selection, "partir" | "copier">,
) {
  const [demande, demander] = createSignal<DemandeDeRangement>();

  /**
   * Range les fils dans le dossier, dès le geste. Si ce dossier a disparu
   * entre-temps, les fils refusés attendent un autre choix.
   */
  async function ranger(
    type: Rangement,
    vue: string,
    cibles: string[],
    dossier: number,
  ) {
    demander(undefined);
    const selection = selectionDe(vue);
    const refuses = await (type === "copier"
      ? selection.copier(cibles, dossier)
      : selection.partir("deplacer", cibles, dossier));
    if (!refuses.length) return;
    await dossiers.recharger();
    const catalogue = dossiers.valeur();
    // Catalogue illisible ou dossier toujours là : le refus vient d’ailleurs.
    if (!catalogue || catalogue.some((d) => d.id === dossier)) return;
    demander({ type, vue, cibles: refuses, disparu: true, apres() {} });
  }

  return {
    demande,
    ranger,
    /** Demande dans quel dossier ranger ces fils. */
    proposer(
      type: Rangement,
      vue: string,
      cibles: string[],
      apres: () => void,
    ) {
      demander({ type, vue, cibles, disparu: false, apres });
    },
    renoncer() {
      demander(undefined);
    },
  };
}
export type Rangements = ReturnType<typeof creerRangements>;
