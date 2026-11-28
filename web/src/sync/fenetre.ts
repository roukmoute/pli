import { createSignal } from "solid-js";
import type { Tranche } from "./pages-api";
import { comparerFils } from "./projection-tri";
import type { Etat } from "./ressource";
import type { Fil, PageFils } from "./vues";

export type LecteurDePage = (tranche?: Tranche) => Promise<PageFils>;

export function filsDe(page: PageFils): Fil[] {
  return [...page.nouveaux, ...page.deja_vus];
}

// Fusion par identifiant stable : la version du dernier lot l’emporte.
export function fusionnerFils(...lots: Fil[][]): Fil[] {
  const parId = new Map<string, Fil>();
  for (const fil of lots.flat()) parId.set(fil.id, fil);
  return [...parId.values()].sort(comparerFils);
}

function ranger(page: PageFils, fils: Fil[]): PageFils {
  return {
    ...page,
    nouveaux: fils.filter((f) => f.non_lu),
    deja_vus: fils.filter((f) => !f.non_lu),
  };
}

// Un fil se trouve après un lot quand il suit chacune de ses lignes.
function suit(fil: Fil, lot: PageFils) {
  return filsDe(lot).every((autre) => comparerFils(fil, autre) > 0);
}

export function etendre(fenetre: PageFils, suite: PageFils): PageFils {
  return ranger(
    { ...suite, revision: Math.min(fenetre.revision, suite.revision) },
    fusionnerFils(filsDe(fenetre), filsDe(suite)),
  );
}

// La relecture fait foi jusqu’à sa dernière ligne. Les pages chargées
// au-delà pendant qu’elle voyageait restent en place.
export function integrer(fenetre: PageFils, frais: PageFils): PageFils {
  const auDela = frais.curseur_suivant
    ? filsDe(fenetre).filter((fil) => suit(fil, frais))
    : [];
  if (!auDela.length) return frais;
  return ranger(
    { ...frais, curseur_suivant: fenetre.curseur_suivant },
    fusionnerFils(auDela, filsDe(frais)),
  );
}

async function lireJusquA(lire: LecteurDePage, connus: Fil[]) {
  let frais = await lire();
  let stable = true;
  while (frais.curseur_suivant && connus.some((fil) => suit(fil, frais))) {
    const reste = connus.length - filsDe(frais).length;
    const page = await lire({
      curseur: frais.curseur_suivant,
      taille: Math.max(0, Math.min(reste, 100)),
    });
    stable = stable && page.revision === frais.revision;
    frais = etendre(frais, page);
  }
  return { frais, stable };
}

// Relit depuis le haut toutes les pages déjà chargées. Si la boîte a changé
// entre deux requêtes, un fil a pu passer d’une page à l’autre : on relit une fois.
export async function relire(
  lire: LecteurDePage,
  connue?: PageFils,
): Promise<PageFils> {
  const connus = connue ? filsDe(connue) : [];
  const essai = await lireJusquA(lire, connus);
  return essai.stable ? essai.frais : (await lireJusquA(lire, connus)).frais;
}

interface Source {
  valeur: () => PageFils | undefined;
  completer: (maj: (fenetre: PageFils) => PageFils) => void;
  charger: () => Promise<void>;
}

export function creerSuite(source: Source, lire: LecteurDePage) {
  const [enCours, occuper] = createSignal(false);
  const [echec, signaler] = createSignal(false);
  let tache: Promise<void> | undefined;
  // La page à peine ajoutée, la liste peut déjà demander la suivante.
  function liberer() {
    tache = undefined;
    occuper(false);
  }
  function charger(): Promise<void> {
    if (tache) return tache;
    const fenetre = source.valeur();
    if (!fenetre?.curseur_suivant) return Promise.resolve();
    occuper(true);
    signaler(false);
    tache = lire({ curseur: fenetre.curseur_suivant }).then(
      (page) => {
        liberer();
        source.completer((actuelle) => etendre(actuelle, page));
        // Une révision différente annonce du courrier arrivé plus haut.
        if (page.revision !== fenetre.revision) void source.charger();
      },
      () => {
        liberer();
        signaler(true);
      },
    );
    return tache;
  }
  return { charger, enCours, echec };
}
export type Suite = ReturnType<typeof creerSuite>;

// Compte les fils passés devant le plus récent déjà vu en haut de liste.
export function creerArrivees(etat: () => Etat<PageFils>) {
  const [vu, retenir] = createSignal<Fil>();
  function fils() {
    const e = etat();
    return e.statut === "pret" ? fusionnerFils(filsDe(e.valeur)) : [];
  }
  return {
    nombre() {
      const repere = vu();
      return repere
        ? fils().filter((fil) => comparerFils(fil, repere) < 0).length
        : 0;
    },
    voir() {
      retenir(fils()[0]);
    },
  };
}
export type Arrivees = ReturnType<typeof creerArrivees>;
