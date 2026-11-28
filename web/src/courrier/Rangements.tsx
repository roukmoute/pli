import { createSignal, For, onSettled, Show } from "solid-js";
import { t } from "../i18n";
import type { Courrier, Dossier, Vue } from "../sync";
import type { DemandeDeRangement } from "../sync/rangements";
import { dossiersDeDestination } from "../sync/roles";
import { ChoixDossier, nomDossier } from "./Archives";
import styles from "./courrier.module.css";

/** Demande le dossier d’un déplacement ou d’une copie, en citant le nombre de fils. */
export function ChoixRangement(props: {
  courrier: Courrier;
  demande: DemandeDeRangement;
}) {
  const nombre = () => props.demande.cibles.length;
  const titre = () =>
    `${t(props.demande.type === "copier" ? "copy" : "move")} ${nombre()} ${t(
      nombre() === 1 ? "thread" : "threads",
    )}`;
  function choisir(dossier: number) {
    const { type, vue, cibles, apres } = props.demande;
    void props.courrier.rangements.ranger(type, vue, cibles, dossier);
    // Les fils déplacés ont quitté leur liste. Les fils copiés y restent.
    if (type === "deplacer") apres();
  }
  return (
    <ChoixDossier
      titre={titre()}
      aide={t("destinationHelp")}
      vide={t("restoreNoFolder")}
      dossiers={dossiersDeDestination(
        props.courrier.dossiers.etat(),
        props.demande.vue,
      )}
      alerte={props.demande.disparu && t("destinationGone")}
      choisir={choisir}
      renoncer={() => props.courrier.rangements.renoncer()}
    />
  );
}

/**
 * Sélecteur de dossiers affiché pendant un glisser : déposer des fils sur un dossier
 * les y déplace, ou les y copie avec Ctrl ou Cmd.
 */
export function DepotDossiers(props: {
  courrier: Courrier;
  vue: Vue;
  /** Fils glissés. */
  cibles: string[];
  deposer: () => void;
}) {
  const [vise, viser] = createSignal<{ dossier: Dossier; copie: boolean }>();
  // Un glisser peut précéder tout autre geste : le catalogue se charge alors ici.
  onSettled(() => props.courrier.archives.preparer());
  const copie = (e: DragEvent) => e.ctrlKey || e.metaKey;
  function survoler(e: DragEvent, dossier: Dossier) {
    e.preventDefault();
    viser({ dossier, copie: copie(e) });
  }
  function lacher(e: DragEvent, dossier: Dossier) {
    e.preventDefault();
    const type = copie(e) ? "copier" : "deplacer";
    void props.courrier.rangements.ranger(
      type,
      props.vue,
      props.cibles,
      dossier.id,
    );
    props.deposer();
  }
  return (
    <aside class={styles.depot} aria-label={t("dropTitle")}>
      <p role="status">
        <Show when={vise()} fallback={t("dropHint")}>
          {(cible) => (
            <>
              {t(cible().copie ? "copyTo" : "moveTo")}{" "}
              {nomDossier(cible().dossier)}
            </>
          )}
        </Show>
      </p>
      <ul>
        <For
          each={dossiersDeDestination(
            props.courrier.dossiers.etat(),
            props.vue,
          )}
          keyed={(d) => d.id}
        >
          {(dossier) => (
            <li
              class={styles.depotDossier}
              data-vise={
                vise()?.dossier.id === dossier().id ? "oui" : undefined
              }
              onDragOver={(e) => survoler(e, dossier())}
              onDragLeave={() => viser(undefined)}
              onDrop={(e) => lacher(e, dossier())}
            >
              {nomDossier(dossier())}
            </li>
          )}
        </For>
      </ul>
    </aside>
  );
}
