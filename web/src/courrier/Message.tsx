import { createSignal, For, onSettled, Show } from "solid-js";
import { t } from "../i18n";
import {
  type Contenu,
  type Entete,
  imageIntegree,
  type Lectures,
} from "../sync/lecture";
import { documentMessage } from "./document-message";
import styles from "./lecture.module.css";

function Rendu(props: { id: string; contenu: Contenu }) {
  const [distantes, autoriser] = createSignal(false);
  const [images, publier] = createSignal<ReadonlyMap<number, string>>(
    new Map(),
  );
  onSettled(() => {
    const chargement = new AbortController();
    const parties = new Set(
      props.contenu.images.flatMap((i) =>
        i.type === "integree" ? [i.partie] : [],
      ),
    );
    if (parties.size) {
      void Promise.allSettled(
        [...parties].map(
          async (partie) =>
            [
              partie,
              await imageIntegree(props.id, partie, chargement.signal),
            ] as const,
        ),
      ).then((resultats) => {
        if (!chargement.signal.aborted)
          publier(
            new Map(
              resultats.flatMap((r) =>
                r.status === "fulfilled" ? [r.value] : [],
              ),
            ),
          );
      });
    }
    return () => chargement.abort();
  });
  return (
    <>
      <Show
        when={
          !distantes() &&
          props.contenu.images.some((i) => i.type === "distante")
        }
      >
        <button
          class={styles.distantes}
          type="button"
          onClick={() => autoriser(true)}
        >
          {t("externalImages")}
        </button>
      </Show>
      <Show when={documentMessage(props.contenu, distantes(), images())} keyed>
        {(document) => (
          <iframe
            class={styles.corps}
            data-court={
              props.contenu.images.length === 0 &&
              props.contenu.html.length <= 180
                ? "true"
                : "false"
            }
            title={t("body")}
            sandbox=""
            referrerpolicy="no-referrer"
            srcdoc={document}
          />
        )}
      </Show>
      <Show when={props.contenu.pieces.length}>
        <ul class={styles.pieces} aria-label={t("attachments")}>
          <For each={props.contenu.pieces}>
            {(piece) => (
              <li>
                <strong>{piece.nom || t("attachment")}</strong>{" "}
                <span>
                  {piece.mime} · {piece.taille.toLocaleString("fr")} o
                </span>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </>
  );
}

export function Message(props: {
  message: Entete;
  fil: string;
  cache: Lectures;
}) {
  const ressource = props.cache.corps(props.message.id);
  const [echec, signaler] = createSignal(false);
  const disponible = () => {
    const etat = ressource.etat();
    return etat.statut === "pret" && etat.valeur.etat === "disponible"
      ? etat.valeur.contenu
      : undefined;
  };
  function marquer() {
    signaler(false);
    void props.cache
      .ouvrir(props.message.id, props.fil)
      .catch(() => signaler(true));
  }
  onSettled(() => {
    if (!disponible()) void ressource.charger();
    if (props.message.non_lu) marquer();
  });
  return (
    <div class={styles.lecture}>
      <Show when={echec()}>
        <p role="status">
          {t("readUnconfirmed")}{" "}
          <button type="button" onClick={marquer}>
            {t("retry")}
          </button>
        </p>
      </Show>
      <Show
        when={disponible()}
        keyed
        fallback={
          <Show
            when={ressource.etat().statut !== "chargement"}
            fallback={<p role="status">{t("messageLoading")}</p>}
          >
            <p role="status">
              {t("availableOnline")}{" "}
              <button type="button" onClick={() => void ressource.charger()}>
                {t("retry")}
              </button>
            </p>
          </Show>
        }
      >
        {(contenu) => <Rendu id={props.message.id} contenu={contenu} />}
      </Show>
    </div>
  );
}
