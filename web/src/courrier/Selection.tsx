import { flush, Show } from "solid-js";
import { t } from "../i18n";
import type { Selection } from "../sync/selection";
import styles from "./courrier.module.css";

export function BarreSelection(props: { ids: string[]; selection: Selection }) {
  const selection = props.selection;
  const lot = () => props.ids.filter((id) => selection.choisis().has(id));
  const nombre = () => lot().length;
  const total = () => props.ids.length;
  let controlePage!: HTMLInputElement;
  function marquer(valeur: boolean, bouton: HTMLButtonElement) {
    void selection.marquer(lot(), "lu", valeur);
    // Le lot quitte la sélection dès le geste et ses commandes disparaissent avec lui.
    flush();
    if (!bouton.isConnected && document.activeElement === document.body)
      controlePage.focus({ preventScroll: true });
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
      </span>
      <Show when={nombre()}>
        <button
          class={styles.boutonLot}
          type="button"
          onClick={(e) => marquer(true, e.currentTarget)}
        >
          {t("markRead")}
        </button>
        <button
          class={styles.boutonLot}
          type="button"
          onClick={(e) => marquer(false, e.currentTarget)}
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
