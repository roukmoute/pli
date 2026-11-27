import { createSignal } from "solid-js";
import { request } from "./api";
import type { Vue } from "./vues";

export type Indicateur = "lu" | "etoile";
interface Resultat {
  id: string;
  etat: "confirme" | "partiel" | "absent" | "non_confirme";
}

export function creerSelection(
  vue: Vue,
  actualiser: (ids: string[], indicateur: Indicateur) => Promise<void>,
) {
  const [choisis, choisir] = createSignal<ReadonlySet<string>>(new Set());
  const [enCours, patienter] = createSignal(false);
  let actif = false;
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
    if (!ids.length || actif) return;
    actif = true;
    patienter(true);
    signaler(0);
    try {
      const reponse = await request("/messages/flags", "POST", {
        ids,
        vue,
        indicateur,
        valeur,
      });
      const resultats: Resultat[] = await reponse.json();
      const confirmes = new Set(
        resultats.filter((r) => r.etat === "confirme").map((r) => r.id),
      );
      const reussis = ids.filter((id) => confirmes.has(id));
      if (indicateur === "lu") cocher(reussis, false);
      signaler(ids.length - reussis.length);
    } catch {
      signaler(ids.length);
    } finally {
      try {
        await actualiser(ids, indicateur);
      } finally {
        actif = false;
        patienter(false);
      }
    }
  }
  return { choisis, cocher, marquer, enCours, echecs };
}
export type Selection = ReturnType<typeof creerSelection>;
