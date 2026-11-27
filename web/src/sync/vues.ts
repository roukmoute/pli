import { createSignal } from "solid-js";
import { getJson } from "./api";

export type Vue = "reception" | "flux" | "justificatifs" | `dossier/${number}`;
export type Section = "nouveaux" | "deja_vus";

export interface Fil {
  id: string;
  objet: string;
  expediteur: { nom: string | null; adresse: string | null };
  extrait: string;
  date: number;
  nombre_messages: number;
  non_lu: boolean;
  etoile: boolean;
  piece_jointe: boolean;
  repondu: boolean;
  transfere: boolean;
}

export interface PageFils {
  compte: number;
  vue?: string;
  dossier?: number;
  nouveaux: Fil[];
  deja_vus: Fil[];
  revision: number;
  expediteurs_en_attente: number;
  curseur_suivant: string | null;
  couverture: { import_en_cours: boolean; proportion_indexee: number | null };
  compteurs: { reception: number; flux: number; justificatifs: number };
}

export interface Dossier {
  id: number;
  path: string;
  delimiter: string | null;
  role: "inbox" | "sent" | "drafts" | "junk" | "trash" | "archive" | null;
  hidden: boolean;
}

type Etat<T> =
  | { statut: "chargement" | "erreur" }
  | { statut: "pret"; valeur: T; perime: boolean };

function ressource<T>(lire: () => Promise<T>) {
  const [etat, publier] = createSignal<Etat<T>>({ statut: "chargement" });
  let enCours: Promise<void> | undefined;
  function charger(): Promise<void> {
    if (enCours) return enCours;
    const precedent = etat();
    if (precedent.statut === "erreur") publier({ statut: "chargement" });
    enCours = lire()
      .then(
        (valeur) => {
          publier({ statut: "pret", valeur, perime: false });
        },
        () => {
          publier(
            precedent.statut === "pret"
              ? { ...precedent, perime: true }
              : { statut: "erreur" },
          );
        },
      )
      .finally(() => {
        enCours = undefined;
      });
    return enCours;
  }
  return { etat, charger };
}

function estDossier(vue: Vue): vue is `dossier/${number}` {
  return vue.startsWith("dossier/");
}

function lirePage(compte: number, vue: Vue): Promise<PageFils> {
  const id = estDossier(vue) ? Number(vue.slice("dossier/".length)) : undefined;
  const chemin =
    id === undefined
      ? (`/views/${vue}` as const)
      : (`/folders/${id}/messages` as const);
  return getJson(chemin, (corps) => {
    const page = corps as PageFils | null;
    if (
      page?.compte !== compte ||
      (id === undefined ? page.vue !== vue : page.dossier !== id) ||
      typeof page.couverture?.import_en_cours !== "boolean" ||
      !Number.isInteger(page.expediteurs_en_attente) ||
      !Array.isArray(page.nouveaux) ||
      !Array.isArray(page.deja_vus)
    ) {
      throw new Error("La liste reçue ne correspond pas à la boîte ouverte.");
    }
    return page;
  });
}

function lireDossiers(): Promise<Dossier[]> {
  return getJson("/folders", (corps) => {
    const liste = corps as { folders?: Dossier[] } | null;
    if (!Array.isArray(liste?.folders))
      throw new Error("Liste de dossiers illisible.");
    return liste.folders.filter((dossier) => !dossier.hidden);
  });
}

interface Repere {
  defilement: number;
  section: Section;
  fil?: string;
  ancre?: { fil: string; decalage: number };
}

function nouvelleVue(compte: number, vue: Vue) {
  const repere: Repere = { defilement: 0, section: "nouveaux" };
  return { ...ressource(() => lirePage(compte, vue)), repere };
}

// Chaque session affichée possède son propre cache. Une réponse tardive d’un autre
// compte reste liée à l’ancienne instance, même si les deux boîtes ont une vue du même nom.
export function creerCourrier(compte: number) {
  const vues = new Map<Vue, ReturnType<typeof nouvelleVue>>();
  const dossiers = ressource(lireDossiers);
  function obtenirVue(vue: Vue) {
    let entree = vues.get(vue);
    if (!entree) {
      entree = nouvelleVue(compte, vue);
      vues.set(vue, entree);
    }
    return entree;
  }
  return {
    dossiers,
    obtenirVue,
    chargerVue: (vue: Vue) => obtenirVue(vue).charger(),
  };
}

export type Courrier = ReturnType<typeof creerCourrier>;
