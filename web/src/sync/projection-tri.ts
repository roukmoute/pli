import type { IntentionTri } from "./tri-api";
import type { Fil, PageFils, Vue } from "./vues";

function cible(vue: Vue, action: IntentionTri, inbox: boolean) {
  return action.choix === "accepte" && (inbox || vue === action.destination);
}

function fusionner(ancien: Fil, ajoute: Fil): Fil {
  const dernier = ajoute.date >= ancien.date ? ajoute : ancien;
  return {
    ...dernier,
    nombre_messages: ancien.nombre_messages + ajoute.nombre_messages,
    non_lu: ancien.non_lu || ajoute.non_lu,
    etoile: ancien.etoile || ajoute.etoile,
    repondu: ancien.repondu || ajoute.repondu,
    transfere: ancien.transfere || ajoute.transfere,
    piece_jointe: ancien.piece_jointe || ajoute.piece_jointe,
  };
}

export function comparerFils(a: Fil, b: Fil) {
  if (a.date !== b.date) return b.date - a.date;
  if (a.id === b.id) return 0;
  return a.id < b.id ? 1 : -1;
}

export function projeterTri(
  page: PageFils,
  vue: Vue,
  action: IntentionTri,
  inbox: boolean,
): PageFils {
  const { entree } = action;
  const attente =
    entree.etat === "en_attente"
      ? Math.max(0, page.expediteurs_en_attente - 1)
      : page.expediteurs_en_attente;
  if (vue.startsWith("dossier/") && !inbox)
    return { ...page, expediteurs_en_attente: attente };
  const admis = cible(vue, action, inbox);
  const presents = [...page.nouveaux, ...page.deja_vus];
  const fils = new Map(
    presents
      .filter((f) => f.expediteur.adresse !== entree.adresse)
      .map((f) => [f.id, f]),
  );
  if (admis) {
    for (const fil of entree.apercus) {
      const actuel = fils.get(fil.id);
      fils.set(fil.id, actuel ? fusionner(actuel, fil) : fil);
    }
  }
  const premiere = Array.from(fils.values()).sort(comparerFils).slice(0, 20);
  return {
    ...page,
    expediteurs_en_attente: attente,
    nouveaux: premiere.filter((f) => f.non_lu),
    deja_vus: premiere.filter((f) => !f.non_lu),
  };
}

export function amorcerVue(
  compte: number,
  vue: Vue,
  intentions: IntentionTri[],
  inbox: boolean,
  attente: number,
): PageFils | undefined {
  if (!intentions.some((intention) => cible(vue, intention, inbox))) return;
  const portee = vue.startsWith("dossier/")
    ? { dossier: Number(vue.slice("dossier/".length)) }
    : { vue };
  return {
    compte,
    ...portee,
    nouveaux: [],
    deja_vus: [],
    revision: 0,
    expediteurs_en_attente: attente,
    curseur_suivant: null,
    couverture: { import_en_cours: false, proportion_indexee: null },
    compteurs: { reception: 0, flux: 0, justificatifs: 0 },
  };
}
