import { For, onSettled, Show } from "solid-js";
import { t } from "../i18n";
import { Icone } from "../icons/Icone";
import type { Archives } from "../sync/archives";
import styles from "./courrier.module.css";

// Le catalogue des dossiers se charge dès que la commande s’affiche :
// le geste d’archivage n’attendra pas le réseau.
export function CommandeArchiver(props: {
  archives: Archives;
  lancer: () => void;
  /** Dit la portée de l’action : message, fil ou sélection. */
  libelle: string;
  class?: string;
  icone?: boolean;
}) {
  onSettled(() => props.archives.preparer());
  return (
    <button
      type="button"
      class={props.class}
      aria-label={props.libelle}
      onClick={() => props.lancer()}
    >
      <Show when={props.icone}>
        <Icone nom="archive" libelle={props.libelle} />
      </Show>
      {props.libelle}
    </button>
  );
}

/** Demande le dossier qui recevra les archives quand la messagerie n’en annonce aucun. */
export function ChoixArchives(props: {
  archives: Archives;
  lancer: () => void;
}) {
  let dialogue!: HTMLDialogElement;
  onSettled(() => {
    const origine = document.activeElement as HTMLElement;
    dialogue.showModal();
    // À la fermeture, le focus retrouve la commande qui a ouvert la demande.
    return () => origine.focus({ preventScroll: true });
  });
  return (
    <dialog
      ref={dialogue}
      class={styles.choix}
      aria-labelledby="titre-archives"
      onCancel={(e) => {
        e.preventDefault();
        props.archives.renoncer();
      }}
    >
      <h2 id="titre-archives">{t("archiveDestination")}</h2>
      <p>{t("archiveDestinationHelp")}</p>
      <Show
        when={props.archives.candidats().length}
        fallback={<p>{t("archiveNoFolder")}</p>}
      >
        <ul>
          <For each={props.archives.candidats()} keyed={(d) => d.id}>
            {(dossier) => (
              <li>
                <button
                  type="button"
                  class={styles.choixDossier}
                  onClick={() =>
                    void props.archives.choisir(dossier().id, props.lancer)
                  }
                >
                  {dossier().path}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.archives.echec()}>
        <p role="alert">{t("archiveDestinationError")}</p>
      </Show>
      <button type="button" onClick={() => props.archives.renoncer()}>
        {t("cancel")}
      </button>
    </dialog>
  );
}
