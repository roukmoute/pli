import { flush, Show } from "solid-js";
import { t } from "../i18n";
import type { Archives } from "../sync/archives";
import type { Selection } from "../sync/selection";
import { CommandeArchiver } from "./Archives";
import styles from "./courrier.module.css";

export function BarreSelection(props: {
  ids: string[];
  selection: Selection;
  archives: Archives;
  /** Faux dans le dossier qui reçoit les archives. */
  archivable: boolean;
  /** Fils que la messagerie n’a pas fini d’archiver depuis cette liste. */
  enPartance: number;
  /** Reçoit le focus quand la liste s’est vidée. */
  titre: HTMLElement;
}) {
  const selection = props.selection;
  const lot = () => props.ids.filter((id) => selection.choisis().has(id));
  const nombre = () => lot().length;
  const total = () => props.ids.length;
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
  function archiver() {
    props.archives.avecDestination(() => {
      void selection.archiver(lot());
      rendreFocus();
    });
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
        <Show when={props.enPartance}>
          {(n) => (
            <span class={styles.partance}>
              {t("archivePending")} {n()} {t(n() === 1 ? "thread" : "threads")}.
            </span>
          )}
        </Show>
      </span>
      <Show when={nombre()}>
        <Show when={props.archivable}>
          <CommandeArchiver
            class={styles.boutonLot}
            archives={props.archives}
            lancer={archiver}
            libelle={t("archiveSelection")}
          />
        </Show>
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
      <Show when={selection.echecs()}>
        {(n) => (
          <span role="alert">
            {t("flagsUnconfirmed")} {n()} {t(n() === 1 ? "thread" : "threads")}.
          </span>
        )}
      </Show>
    </div>
  );
}
