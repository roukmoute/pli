import { createSignal, Match, Show, Switch } from "solid-js";
import { t } from "../i18n";
import type { Courrier, Dossier, Vue } from "../sync";
import { filsDe } from "../sync/fenetre";
import { dossierDeVue, dossiersDeRestauration } from "../sync/roles";
import type { Depart } from "../sync/selection";
import { ChoixDossier, CommandeArchiver, Confirmation } from "./Archives";

export type Portee = "selection" | "fil" | "message";

// Chaque commande dit sa portée : la sélection, le fil ou le message.
const libelles = {
  archiver: {
    selection: "archiveSelection",
    fil: "archiveThread",
    message: "archiveMessage",
  },
  corbeille: {
    selection: "trashSelection",
    fil: "trashThread",
    message: "trashMessage",
  },
  restaurer: {
    selection: "restoreSelection",
    fil: "restoreThread",
    message: "restoreMessage",
  },
} as const;

/** « 2 fils seront supprimés définitivement de Corbeille. » */
export function porteeDefinitive(nombre: number, dossier: Dossier) {
  const fils = t(nombre === 1 ? "deleteForeverOne" : "deleteForeverMany");
  return `${nombre} ${fils} ${dossier.path}.`;
}

/**
 * Commandes qui font quitter leur liste aux fils : archiver et supprimer, ou restaurer
 * et supprimer définitivement quand la vue est la Corbeille.
 */
export function CommandesDepart(props: {
  courrier: Courrier;
  vue: Vue;
  /** Fils visés au moment du geste. */
  cibles: () => string[];
  portee: Portee;
  /** Après le lancement : replace le focus ou quitte le fil ouvert. */
  apres: () => void;
  class?: string;
}) {
  const [demande, demander] = createSignal<"confirmation" | "dossier">();
  const dossier = () => dossierDeVue(props.courrier.dossiers.etat(), props.vue);
  const role = () => dossier()?.role;
  const entree = () => props.courrier.obtenirVue(props.vue);

  function partir(type: Depart, arrivee?: number) {
    demander(undefined);
    void entree().selection.partir(type, props.cibles(), arrivee);
    props.apres();
  }
  function restaurer() {
    const cibles = new Set(props.cibles());
    const page = entree().source.valeur();
    // Un fil sans dossier d’origine attend que l’utilisateur dise où le remettre.
    const inconnue =
      page &&
      filsDe(page).some((fil) => cibles.has(fil.id) && fil.sans_origine);
    if (inconnue) demander("dossier");
    else partir("restaurer");
  }
  return (
    <>
      <Switch>
        <Match when={role() === "trash"}>
          <button type="button" class={props.class} onClick={restaurer}>
            {t(libelles.restaurer[props.portee])}
          </button>
          <button
            type="button"
            class={props.class}
            onClick={() => demander("confirmation")}
          >
            {t("deleteForever")}
          </button>
        </Match>
        <Match when={true}>
          <Show when={role() !== "archive"}>
            <CommandeArchiver
              class={props.class}
              archives={props.courrier.archives}
              lancer={() =>
                props.courrier.archives.avecDestination(() =>
                  partir("archiver"),
                )
              }
              libelle={t(libelles.archiver[props.portee])}
            />
          </Show>
          <button
            type="button"
            class={props.class}
            onClick={() => partir("corbeille")}
          >
            {t(libelles.corbeille[props.portee])}
          </button>
        </Match>
      </Switch>
      <Show when={demande() === "confirmation" && dossier()}>
        {(corbeille) => (
          <Confirmation
            titre={t("deleteForever")}
            portee={porteeDefinitive(props.cibles().length, corbeille())}
            confirmer={() => partir("supprimer_definitivement")}
            renoncer={() => demander(undefined)}
          />
        )}
      </Show>
      <Show when={demande() === "dossier"}>
        <ChoixDossier
          titre={t("restoreDestination")}
          aide={t("restoreDestinationHelp")}
          vide={t("restoreNoFolder")}
          dossiers={dossiersDeRestauration(props.courrier.dossiers.etat())}
          choisir={(arrivee) => partir("restaurer", arrivee)}
          renoncer={() => demander(undefined)}
        />
      </Show>
    </>
  );
}
