import { createSignal, For, flush, Show } from "solid-js";
import { type CleLibelle, t } from "../i18n";
import type { Courrier, Vue } from "../sync";
import type { Partance } from "../sync/departs";
import { dossierDeVue } from "../sync/roles";
import type { Selection } from "../sync/selection";
import { Confirmation } from "./Archives";
import { CommandesDepart, porteeDefinitive } from "./Commandes";
import styles from "./courrier.module.css";

// Annonce de chaque départ que la messagerie n’a pas encore confirmé.
const attentes: Partial<Record<Partance["type"], CleLibelle>> = {
  archiver: "archivePending",
  corbeille: "trashPending",
  restaurer: "restorePending",
  supprimer_definitivement: "deletePending",
};

function EnPartance(props: { partance: Partance }) {
  return (
    <span class={styles.partance}>
      <Show when={attentes[props.partance.type]} fallback={t("emptyPending")}>
        {(cle) => (
          <>
            {t(cle())} {props.partance.nombre}{" "}
            {t(props.partance.nombre === 1 ? "thread" : "threads")}.
          </>
        )}
      </Show>
    </span>
  );
}

export function BarreSelection(props: {
  ids: string[];
  selection: Selection;
  courrier: Courrier;
  vue: Vue;
  /** Nombre de fils de la liste entière. */
  total?: number;
  /** Départs que la messagerie n’a pas fini de traiter depuis cette liste. */
  enPartance: Partance[];
  /** Reçoit le focus quand la liste s’est vidée. */
  titre: HTMLElement;
}) {
  const selection = props.selection;
  const lot = () => props.ids.filter((id) => selection.choisis().has(id));
  const nombre = () => lot().length;
  const total = () => props.ids.length;
  const [vidage, proposerVidage] = createSignal(false);
  // La Corbeille et les Indésirables se vident d’un geste, confirmé.
  const videable = () => {
    const dossier = dossierDeVue(props.courrier.dossiers.etat(), props.vue);
    const jetable = dossier?.role === "trash" || dossier?.role === "junk";
    return jetable && props.total ? dossier : undefined;
  };
  let controlePage!: HTMLInputElement;
  // Le lot quitte la sélection dès le geste et ses commandes disparaissent avec lui.
  // Le focus qu’elles portaient rejoint la case de la page, ou le titre d’une liste vidée.
  function rendreFocus() {
    flush();
    if (document.activeElement !== document.body) return;
    const cible = controlePage.isConnected ? controlePage : props.titre;
    cible.focus({ preventScroll: true });
  }
  function marquer(valeur: boolean) {
    void selection.marquer(lot(), "lu", valeur);
    rendreFocus();
  }
  function vider() {
    proposerVidage(false);
    void selection.vider(props.vue);
    rendreFocus();
  }
  return (
    <div class={styles.selection}>
      <Show when={total()}>
        <label class={styles.touteLaPage}>
          <input
            ref={controlePage}
            type="checkbox"
            checked={nombre() === total()}
            prop:indeterminate={nombre() > 0 && nombre() < total()}
            onChange={(e) =>
              selection.cocher(props.ids, e.currentTarget.checked)
            }
          />
          {t("selectPage")}
        </label>
      </Show>
      <span role="status" aria-live="polite" aria-atomic="true">
        <Show when={total()}>
          {nombre()} {t(nombre() === 1 ? "selectedThread" : "selectedThreads")}{" "}
          {t("selectionOf")} {total()}
        </Show>
        <For each={props.enPartance} keyed={(partance) => partance.type}>
          {(partance) => <EnPartance partance={partance()} />}
        </For>
      </span>
      <Show when={nombre()}>
        <CommandesDepart
          class={styles.boutonLot}
          courrier={props.courrier}
          vue={props.vue}
          cibles={lot}
          portee="selection"
          apres={rendreFocus}
        />
        <button
          class={styles.boutonLot}
          type="button"
          onClick={() => marquer(true)}
        >
          {t("markRead")}
        </button>
        <button
          class={styles.boutonLot}
          type="button"
          onClick={() => marquer(false)}
        >
          {t("markUnread")}
        </button>
      </Show>
      <Show when={videable()}>
        {(dossier) => (
          <>
            <button
              class={styles.boutonLot}
              type="button"
              onClick={() => proposerVidage(true)}
            >
              {t("emptyFolder")}
            </button>
            <Show when={vidage()}>
              <Confirmation
                titre={t("emptyFolder")}
                portee={porteeDefinitive(props.total as number, dossier())}
                confirmer={vider}
                renoncer={() => proposerVidage(false)}
              />
            </Show>
          </>
        )}
      </Show>
      <Show when={selection.echecs()}>
        {(n) => (
          <span role="alert">
            {t("flagsUnconfirmed")} {n()} {t(n() === 1 ? "thread" : "threads")}.
          </span>
        )}
      </Show>
      <Show when={selection.vidageRefuse()}>
        <span role="alert">{t("emptyUnconfirmed")}</span>
      </Show>
    </div>
  );
}
