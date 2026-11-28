import { createSignal } from "solid-js";
import { ApiError, request } from "./api";

export type TypeOperation = "etoile" | "sans_etoile" | "lu" | "non_lu";
type EtatCible = "en_attente" | "confirme" | "partiel" | "refuse" | "absent";

export interface Operation {
  id: string;
  type: TypeOperation;
  etat: "en_attente" | "confirmee" | "refusee";
  revision: number;
  resultats: { id: string; etat: EtatCible }[];
}

/** Action demandée : son type, la vue où ses cibles sont désignées et ces cibles. */
export interface Intention {
  type: TypeOperation;
  vue: string;
  cibles: string[];
}

/** Intention envoyée sous un identifiant stable, pas encore confirmée ni refusée. */
export interface EnAttente extends Intention {
  id: string;
}

/**
 * Reçoit les cibles confirmées et la révision du compte après l’écriture.
 * Corrige le cache sans attendre, puis rend la promesse de ses relectures.
 */
type Conclusion = (
  operation: EnAttente,
  confirmees: string[],
  revision: number,
) => Promise<unknown>;

// Le serveur accepte cent cibles par opération.
const LOT = 100;
const REPRISES = [300, 1200];

function pause(duree: number) {
  return new Promise<void>((reprendre) => setTimeout(reprendre, duree));
}

// Une réponse du serveur autre qu’une panne ne changerait pas au rejeu.
function definitive(erreur: unknown) {
  return erreur instanceof ApiError && erreur.status < 500;
}

export function creerOperations(conclure: Conclusion, patienter = pause) {
  const [enAttente, publier] = createSignal<readonly EnAttente[]>([]);

  // Une réponse perdue se rattrape en renvoyant le même identifiant.
  async function envoyer(operation: EnAttente): Promise<Operation> {
    for (let essai = 0; ; essai++) {
      try {
        const reponse = await request("/operations", "POST", operation);
        return await reponse.json();
      } catch (erreur) {
        if (definitive(erreur) || essai === REPRISES.length) throw erreur;
        await patienter(REPRISES[essai]);
      }
    }
  }

  async function suivre(recue: Operation): Promise<Operation> {
    let operation = recue;
    for (let tour = 0; operation.etat === "en_attente"; tour++) {
      await patienter(Math.min(300 * 2 ** tour, 5000));
      try {
        const reponse = await request(`/operations/${operation.id}`);
        operation = await reponse.json();
      } catch (erreur) {
        // Une coupure passagère laisse l’opération en attente.
        if (definitive(erreur)) throw erreur;
      }
    }
    return operation;
  }

  async function executer(intention: Intention): Promise<string[]> {
    const operation: EnAttente = { ...intention, id: crypto.randomUUID() };
    // L’effet se voit dès maintenant : la projection lit cette file.
    publier((file) => [...file, operation]);
    let confirmees: string[] = [];
    let revision = 0;
    try {
      const recue = await envoyer(operation);
      revision = recue.revision;
      confirmees = (await suivre(recue)).resultats
        .filter((resultat) => resultat.etat === "confirme")
        .map((resultat) => resultat.id);
    } catch {
      // Sans confirmation, toutes les cibles retrouvent leur état d’avant.
    }
    const relectures = conclure(operation, confirmees, revision);
    publier((file) => file.filter((autre) => autre !== operation));
    await relectures;
    return intention.cibles.filter((id) => !confirmees.includes(id));
  }

  /** Applique localement, envoie, puis confirme ou annule. Rend les cibles refusées. */
  async function lancer(intention: Intention): Promise<string[]> {
    const lots = [];
    for (let debut = 0; debut < intention.cibles.length; debut += LOT)
      lots.push(
        executer({
          ...intention,
          cibles: intention.cibles.slice(debut, debut + LOT),
        }),
      );
    return (await Promise.all(lots)).flat();
  }

  return { enAttente, lancer };
}
export type Operations = ReturnType<typeof creerOperations>;
