import { createSignal } from "solid-js";
import { request } from "./api";
import type { ressource } from "./ressource";
import type { Dossier } from "./vues";

type Catalogue = Pick<
  ReturnType<typeof ressource<Dossier[]>>,
  "etat" | "valeur" | "charger" | "corriger"
>;

/** Retient pour le compte le dossier qui reçoit ses archives. Rend le catalogue à jour. */
export async function retenirArchives(dossier: number): Promise<Dossier[]> {
  const reponse = await request("/folders/archive", "PUT", { dossier });
  const catalogue: { folders: Dossier[] } = await reponse.json();
  return catalogue.folders.filter((d) => !d.hidden);
}

function recoit(dossier: Dossier) {
  return dossier.role === "archive";
}

export function creerArchives(dossiers: Catalogue, retenir = retenirArchives) {
  // Action suspendue tant que l’utilisateur n’a pas dit où ranger ses archives.
  const [demande, demander] = createSignal<{ lancer: () => void }>();
  const [echec, signaler] = createSignal(false);

  function decider(action: () => void) {
    const catalogue = dossiers.valeur();
    // Catalogue illisible : le serveur dira si le compte a un dossier d’archives.
    if (!catalogue || catalogue.some(recoit)) return action();
    signaler(false);
    demander({ lancer: action });
  }

  return {
    demande,
    echec,
    /** Charge le catalogue avant le premier geste, pour que ce geste n’attende pas le réseau. */
    preparer() {
      if (!dossiers.valeur()) void dossiers.charger();
    },
    /** Lance l’action si les archives ont un dossier. Sinon, la garde et demande ce dossier. */
    avecDestination(action: () => void) {
      if (dossiers.valeur()) decider(action);
      else void dossiers.charger().then(() => decider(action));
    },
    /** Vrai pour la vue du dossier qui reçoit les archives : rien n’y est à archiver. */
    recoit(vue: string) {
      const etat = dossiers.etat();
      return (
        etat.statut === "pret" &&
        etat.valeur.some((d) => recoit(d) && `dossier/${d.id}` === vue)
      );
    },
    /** Dossiers personnels où l’on peut ranger du courrier. */
    candidats() {
      const etat = dossiers.etat();
      const catalogue = etat.statut === "pret" ? etat.valeur : [];
      return catalogue.filter((d) => d.role === null && d.selectable);
    },
    /** Retient le dossier choisi, puis lance l’action qui attendait. */
    async choisir(dossier: number, action: () => void) {
      signaler(false);
      try {
        const catalogue = await retenir(dossier);
        dossiers.corriger(() => catalogue, catalogue);
      } catch {
        signaler(true);
        return;
      }
      demander(undefined);
      action();
    },
    renoncer() {
      demander(undefined);
    },
  };
}
export type Archives = ReturnType<typeof creerArchives>;
