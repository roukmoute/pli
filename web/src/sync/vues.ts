import { creerArrivees, creerSuite, integrer, relire } from "./fenetre";
import { conclureIndicateur, projeterAttentes } from "./indicateurs";
import { appliquerLecture, creerLectures } from "./lecture";
import { creerOperations, type TypeOperation } from "./operations";
import { lireDossiers, lirePage, type Tranche } from "./pages-api";
import { amorcerVue, projeterTri } from "./projection-tri";
import { type Etat, ressource } from "./ressource";
import { creerSelection } from "./selection";
import { creerTri } from "./tri";
import type { IntentionTri } from "./tri-api";

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
  /** Posé par le navigateur tant que la messagerie n’a pas confirmé l’action. */
  attente?: { etoile?: boolean; lu?: boolean };
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

// L’API sert aujourd’hui un seul ordre et une liste entière.
export type Ordre = "date";
export type Filtre = "tous";

export interface Repere {
  defilement: number;
  section: Section;
  ordre: Ordre;
  filtre: Filtre;
  fil?: string;
  ancre?: { fil: string; decalage: number };
  lecture?: { fil: string; section: Section; ordre: string[] };
}

function nouvelleVue(
  compte: number,
  vue: Vue,
  lancer: (type: TypeOperation, cibles: string[]) => Promise<string[]>,
  afficher: (base: Etat<PageFils>) => Etat<PageFils>,
) {
  const repere: Repere = {
    defilement: 0,
    section: "nouveaux",
    ordre: "date",
    filtre: "tous",
  };
  const lire = (tranche?: Tranche) => lirePage(compte, vue, tranche);
  // La valeur est la fenêtre de toutes les pages chargées pour cette vue.
  const source = ressource(
    (connue?: PageFils) => relire(lire, connue),
    integrer,
  );
  const etat = () => afficher(source.etat());
  return {
    source,
    charger: source.charger,
    etat,
    repere,
    selection: creerSelection(lancer),
    suite: creerSuite(source, lire),
    arrivees: creerArrivees(etat),
  };
}
export type VueCourrier = ReturnType<typeof nouvelleVue>;

// Le cache et les opérations de tri appartiennent à la session affichée.
export function creerCourrier(compte: number) {
  const vues = new Map<Vue, ReturnType<typeof nouvelleVue>>();
  const dossiers = ressource(lireDossiers);
  const tri = creerTri(compte, confirmer);
  const lectures = creerLectures((lecture) => {
    for (const [vue, entree] of vues) {
      if (vue in lecture.non_lus)
        entree.source.corriger((page) => appliquerLecture(page, vue, lecture));
    }
  });
  const operations = creerOperations((operation, confirmees, revision) =>
    conclureIndicateur(vues, lectures, operation, confirmees, revision),
  );

  function estInbox(vue: Vue) {
    const liste = dossiers.etat();
    return (
      liste.statut === "pret" &&
      liste.valeur.some((d) => d.role === "inbox" && `dossier/${d.id}` === vue)
    );
  }
  function compteur() {
    const reception = vues.get("reception")?.source.etat();
    return reception?.statut === "pret"
      ? reception.valeur.expediteurs_en_attente
      : 0;
  }
  function afficher(vue: Vue, etat: Etat<PageFils>): Etat<PageFils> {
    const intentions = tri.intentions();
    if (!intentions.length) return etat;
    const inbox = estInbox(vue);
    const page =
      etat.statut === "pret"
        ? etat.valeur
        : amorcerVue(compte, vue, intentions, inbox, compteur());
    if (!page) return etat;
    const valeur = intentions.reduce(
      (p, i) => projeterTri(p, vue, i, inbox),
      page,
    );
    return {
      statut: "pret",
      valeur,
      perime: etat.statut === "pret" && etat.perime,
    };
  }
  function confirmer(intention: IntentionTri, revision: number) {
    const attente = compteur();
    if (intention.choix === "accepte") obtenirVue(intention.destination);
    for (const [vue, entree] of vues) {
      const inbox = estInbox(vue);
      entree.source.corriger(
        (page) =>
          page.revision < revision
            ? projeterTri(page, vue, intention, inbox)
            : page,
        amorcerVue(compte, vue, [intention], inbox, attente),
      );
      if (
        inbox ||
        vue === intention.entree.destination ||
        vue === intention.destination
      )
        void entree.charger();
    }
  }
  function obtenirVue(vue: Vue) {
    let entree = vues.get(vue);
    if (!entree) {
      entree = nouvelleVue(
        compte,
        vue,
        (type, cibles) => operations.lancer({ type, vue, cibles }),
        // Les actions en attente se superposent à la page servie.
        (etat) =>
          projeterAttentes(afficher(vue, etat), vue, operations.enAttente()),
      );
      vues.set(vue, entree);
    }
    return entree;
  }
  return {
    dossiers,
    lectures,
    tri,
    obtenirVue,
    chargerVue: (vue: Vue) => obtenirVue(vue).charger(),
  };
}

export type Courrier = ReturnType<typeof creerCourrier>;
