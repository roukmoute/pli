import type { JSX } from "@solidjs/web";
import { createSignal, createUniqueId, For, onSettled, Show } from "solid-js";
import { t } from "../i18n";
import type { Dossier } from "../sync";
import type { Archives } from "../sync/archives";
import styles from "./courrier.module.css";
import { normaliser } from "./texte";

// Le catalogue des dossiers se charge dès que la commande s’affiche :
// le geste d’archivage n’attendra pas le réseau.
export function CommandeArchiver(props: {
  archives: Archives;
  lancer: () => void;
  /** Dit la portée de l’action : message, fil ou sélection. */
  libelle: string;
  class?: string;
}) {
  onSettled(() => props.archives.preparer());
  return (
    <button type="button" class={props.class} onClick={() => props.lancer()}>
      {props.libelle}
    </button>
  );
}

/**
 * Boîte de dialogue modale. Le focus y reste, Échap renonce, et la fermeture
 * rend le focus à la commande qui l’a ouverte.
 */
export function Dialogue(props: {
  titre: string;
  renoncer: () => void;
  /** Place le focus sur « Annuler » à l’ouverture, avant une action définitive. */
  prudent?: boolean;
  children: JSX.Element;
}) {
  let dialogue!: HTMLDialogElement;
  const titre = createUniqueId();
  onSettled(() => {
    const origine = document.activeElement as HTMLElement;
    dialogue.showModal();
    return () => origine.focus({ preventScroll: true });
  });
  return (
    <dialog
      ref={dialogue}
      class={styles.choix}
      aria-labelledby={titre}
      onCancel={(e) => {
        e.preventDefault();
        props.renoncer();
      }}
    >
      <h2 id={titre}>{props.titre}</h2>
      {props.children}
      <button
        type="button"
        autofocus={props.prudent}
        onClick={() => props.renoncer()}
      >
        {t("cancel")}
      </button>
    </dialog>
  );
}

// La boîte de réception porte son nom d’usage, les autres dossiers leur chemin.
export function nomDossier(dossier: Dossier) {
  return dossier.role === "inbox" ? t("inbox") : dossier.path;
}

/** Sélecteur de dossiers : ceux du compte ouvert, filtrables au clavier. */
export function ChoixDossier(props: {
  titre: string;
  aide: string;
  /** Dit pourquoi aucun dossier n’est proposé. */
  vide: string;
  dossiers: Dossier[];
  /** Annonce, par exemple le refus du dossier choisi auparavant. */
  alerte?: string | false;
  choisir: (dossier: number) => void;
  renoncer: () => void;
}) {
  const [filtre, filtrer] = createSignal("");
  const retenus = () =>
    props.dossiers.filter((dossier) =>
      normaliser(nomDossier(dossier)).includes(normaliser(filtre())),
    );
  return (
    <Dialogue titre={props.titre} renoncer={props.renoncer}>
      <p>{props.aide}</p>
      <Show when={props.alerte}>
        <p role="alert">{props.alerte}</p>
      </Show>
      <Show when={props.dossiers.length} fallback={<p>{props.vide}</p>}>
        <input
          type="search"
          aria-label={t("destinationFilter")}
          placeholder={t("destinationFilter")}
          value={filtre()}
          onInput={(e) => filtrer(e.currentTarget.value)}
        />
        <Show
          when={retenus().length}
          fallback={<p role="status">{t("destinationNone")}</p>}
        >
          <ul>
            <For each={retenus()} keyed={(d) => d.id}>
              {(dossier) => (
                <li>
                  <button
                    type="button"
                    class={styles.choixDossier}
                    onClick={() => props.choisir(dossier().id)}
                  >
                    {nomDossier(dossier())}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Show>
    </Dialogue>
  );
}

/** Demande le dossier qui recevra les archives quand la messagerie n’en annonce aucun. */
export function ChoixArchives(props: {
  archives: Archives;
  lancer: () => void;
}) {
  return (
    <ChoixDossier
      titre={t("archiveDestination")}
      aide={t("archiveDestinationHelp")}
      vide={t("archiveNoFolder")}
      dossiers={props.archives.candidats()}
      alerte={props.archives.echec() && t("archiveDestinationError")}
      choisir={(dossier) => void props.archives.choisir(dossier, props.lancer)}
      renoncer={() => props.archives.renoncer()}
    />
  );
}

/** Fait confirmer une action définitive en citant sa portée exacte. */
export function Confirmation(props: {
  titre: string;
  portee: string;
  confirmer: () => void;
  renoncer: () => void;
}) {
  return (
    <Dialogue titre={props.titre} renoncer={props.renoncer} prudent>
      <p>{props.portee}</p>
      <button
        type="button"
        class={styles.definitif}
        onClick={() => props.confirmer()}
      >
        {props.titre}
      </button>
    </Dialogue>
  );
}
