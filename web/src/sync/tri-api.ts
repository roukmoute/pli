import { ApiError, getJson, request } from "./api";
import type { Fil } from "./vues";

export type DestinationTri = "reception" | "flux" | "justificatifs";
export type ChoixTri = "accepte" | "refuse";
export type ListeTri = "attente" | "historique";
export interface DecisionTri {
  adresse: string;
  etat: ChoixTri | "en_attente";
  destination: DestinationTri;
  revision: number;
  date_decision: number | null;
}
export interface LettreTri {
  uid: number;
  generation: number;
  fil: string;
  nom: string | null;
  objet: string;
  extrait: string;
  date: number;
  non_lu: boolean;
  etoile: boolean;
  piece_jointe: boolean;
  repondu: boolean;
  transfere: boolean;
}
export interface CurseurTri {
  uid: number;
  generation: number;
}
export interface LettresTri {
  messages: LettreTri[];
  suivant: CurseurTri | null;
}
export interface Expediteur extends DecisionTri, LettresTri {
  apercus: Fil[];
}
export interface CatalogueTri {
  compte: number;
  revision: number;
  expediteurs: Expediteur[];
  suite: string | null;
}
export interface ResultatTri {
  compte: number;
  revision_compte: number;
  decision: DecisionTri;
}
export interface IntentionTri {
  entree: Expediteur;
  choix: ChoixTri;
  destination: DestinationTri;
}

export function lireCatalogue(compte: number, mode: ListeTri, apres = "") {
  const params = new URLSearchParams({ details: "true" });
  if (apres) params.set("apres", apres);
  const fin = mode === "historique" ? "/historique" : "";
  return getJson(`/screener${fin}?${params}`, (valeur) => {
    const liste = valeur as CatalogueTri | null;
    if (liste?.compte !== compte || !Array.isArray(liste.expediteurs))
      throw new Error("Catalogue de tri illisible.");
    return liste;
  });
}

export function lireLettres(adresse: string, curseur: CurseurTri) {
  const params = new URLSearchParams({
    adresse,
    avant: String(curseur.uid),
    generation: String(curseur.generation),
  });
  return getJson(`/screener/messages?${params}`, (valeur) => {
    const lot = valeur as LettresTri | null;
    if (!Array.isArray(lot?.messages))
      throw new Error("Courrier de l’expéditeur illisible.");
    return lot;
  });
}

function resultat(
  compte: number,
  adresse: string,
  valeur: unknown,
): ResultatTri {
  const r = valeur as ResultatTri | null;
  if (
    r?.compte !== compte ||
    r.decision?.adresse !== adresse ||
    !Number.isInteger(r.revision_compte)
  )
    throw new Error("Décision de tri illisible.");
  return r;
}

export async function envoyerDecision(compte: number, intention: IntentionTri) {
  const { entree, choix, destination } = intention;
  try {
    const reponse = await request("/screener/decisions", "POST", {
      adresse: entree.adresse,
      decision: choix,
      destination,
      revision_attendue: entree.revision,
    });
    return {
      conflit: false,
      resultat: resultat(compte, entree.adresse, await reponse.json()),
    };
  } catch (cause) {
    if (cause instanceof ApiError && cause.status === 409)
      return {
        conflit: true,
        resultat: resultat(compte, entree.adresse, cause.details),
      };
    throw cause;
  }
}
