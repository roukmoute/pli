import { createSignal } from "solid-js";
import { ressource } from "./ressource";
import {
  type CatalogueTri,
  type DestinationTri,
  type Expediteur,
  envoyerDecision,
  type IntentionTri,
  type ListeTri,
  lireCatalogue,
  lireLettres,
} from "./tri-api";

type Avis =
  | { type: "conflit"; entree: Expediteur }
  | { type: "erreur"; adresse: string };

function inscrire(catalogue: CatalogueTri, entree: Expediteur): CatalogueTri {
  const expediteurs = [...catalogue.expediteurs];
  const index = expediteurs.findIndex((e) => e.adresse >= entree.adresse);
  if (index < 0) expediteurs.push(entree);
  else if (expediteurs[index].adresse === entree.adresse)
    expediteurs[index] = entree;
  else expediteurs.splice(index, 0, entree);
  return { ...catalogue, expediteurs };
}

export function creerTri(
  compte: number,
  confirmer: (intention: IntentionTri, revision: number) => void,
) {
  const attente = ressource(() => lireCatalogue(compte, "attente"));
  const historique = ressource(() => lireCatalogue(compte, "historique"));
  const [intentions, publier] = createSignal<IntentionTri[]>([]);
  const actifs = new Map<string, IntentionTri>();
  const [avis, annoncer] = createSignal<Avis>();
  const catalogues = { attente, historique };

  async function agir(intention: IntentionTri) {
    const { entree } = intention;
    if (actifs.has(entree.adresse)) return;
    annoncer(undefined);
    actifs.set(entree.adresse, intention);
    publier([...actifs.values()]);
    try {
      const { conflit, resultat } = await envoyerDecision(compte, intention);
      const confirmee = { ...entree, ...resultat.decision };
      const choix = confirmee.etat;
      if (choix === "en_attente") throw new Error("Décision non enregistrée.");
      confirmer(
        { entree, choix, destination: confirmee.destination },
        resultat.revision_compte,
      );
      attente.corriger((c) => ({
        ...c,
        expediteurs: c.expediteurs.filter((e) => e.adresse !== entree.adresse),
      }));
      historique.corriger((c) => inscrire(c, confirmee));
      if (conflit) annoncer({ type: "conflit", entree: confirmee });
    } catch {
      annoncer({ type: "erreur", adresse: entree.adresse });
    } finally {
      actifs.delete(entree.adresse);
      publier([...actifs.values()]);
    }
  }

  async function suite(mode: ListeTri) {
    const catalogue = catalogues[mode];
    const etat = catalogue.etat();
    if (etat.statut !== "pret" || !etat.valeur.suite) return;
    const apres = etat.valeur.suite;
    const suivant = await lireCatalogue(compte, mode, apres);
    if (suivant.revision !== etat.valeur.revision) {
      await catalogue.charger();
      return;
    }
    catalogue.corriger((c) =>
      c.suite === apres
        ? {
            ...suivant,
            expediteurs: [...c.expediteurs, ...suivant.expediteurs],
          }
        : c,
    );
  }

  async function lettresSuivantes(mode: ListeTri, entree: Expediteur) {
    const marque = entree.suivant;
    if (!marque) return;
    const suite = await lireLettres(entree.adresse, marque);
    catalogues[mode].corriger((c: CatalogueTri) => ({
      ...c,
      expediteurs: c.expediteurs.map((e) =>
        e.adresse === entree.adresse && e.suivant === marque
          ? {
              ...e,
              messages: [...e.messages, ...suite.messages],
              suivant: suite.suivant,
            }
          : e,
      ),
    }));
  }

  return {
    attente,
    historique,
    intentions,
    avis,
    fermerAvis: () => annoncer(undefined),
    suite,
    lettresSuivantes,
    accepterExpediteur: (
      entree: Expediteur,
      destination: DestinationTri = "reception",
    ) => agir({ entree, choix: "accepte", destination }),
    refuserExpediteur: (entree: Expediteur) =>
      agir({ entree, choix: "refuse", destination: "reception" }),
  };
}
export type Tri = ReturnType<typeof creerTri>;
