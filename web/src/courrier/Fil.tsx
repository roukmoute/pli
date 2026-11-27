import { useLocation, useNavigate, useParams } from "@solidjs/router";
import { createEffect, createSignal, For, onSettled, Show } from "solid-js";
import { Avatar } from "../components/Avatar";
import { t } from "../i18n";
import { Icone } from "../icons/Icone";
import type { Entete, Lectures } from "../sync/lecture";
import layout from "./courrier.module.css";
import styles from "./lecture.module.css";
import { Message } from "./Message";

const date = new Intl.DateTimeFormat("fr", {
  dateStyle: "medium",
  timeStyle: "short",
});
const actions = [
  ["reply", "repondre"],
  ["replyLater", "plusTard"],
  ["putAside", "deCote"],
  ["more", "plus"],
] as const;
function nom(m: Entete) {
  return m.nom || m.adresse || t("unknownSender");
}
function participants(messages: Entete[]) {
  const noms = new Map<string, string>();
  for (const m of messages) {
    if (m.adresse) noms.set(m.adresse, nom(m));
    for (const c of [
      ...(m.destinataires.to || []),
      ...(m.destinataires.cc || []),
    ])
      if (!noms.has(c.address)) noms.set(c.address, c.name || c.address);
  }
  return [...noms.values()].join(", ");
}
function Carte(props: {
  message: Entete;
  fil: string;
  active: boolean;
  cible?: string;
  cache: Lectures;
}) {
  const lieu = useLocation<{ retour: string }>();
  let carte!: HTMLElement;
  const [choix, basculer] = createSignal<boolean>();
  const ouverte = () => choix() ?? props.active;
  const chemin = () =>
    `/fil/${encodeURIComponent(props.fil)}/message/${encodeURIComponent(props.message.id)}`;
  createEffect(
    () => props.cible,
    (cible) => {
      if (cible !== props.message.id) return;
      basculer(true);
      const zone = carte.closest("main");
      if (!zone) return;
      const bounds = zone.getBoundingClientRect();
      const position = carte.getBoundingClientRect();
      if (position.top < bounds.top || position.top >= bounds.bottom)
        zone.scrollTop += position.top - bounds.top;
    },
  );
  return (
    <article ref={carte} class={styles.carte}>
      <header>
        <button
          type="button"
          class={styles.auteur}
          aria-expanded={ouverte() ? "true" : "false"}
          onClick={() => basculer(!ouverte())}
        >
          <span aria-hidden="true">
            <Avatar nom={nom(props.message)} />
          </span>
          <strong>{nom(props.message)}</strong>
        </button>
        <a
          class={styles.permalien}
          href={chemin()}
          replace
          noscroll
          state={JSON.stringify(lieu.state)}
        >
          <time datetime={new Date(props.message.date * 1000).toISOString()}>
            {date.format(props.message.date * 1000)}
          </time>
        </a>
      </header>
      <Show when={ouverte()}>
        <Message message={props.message} fil={props.fil} cache={props.cache} />
      </Show>
    </article>
  );
}

function Pile(props: { fil: string; id?: string; cache: Lectures }) {
  const entree = props.cache.fil(props.fil);
  const conversation = () => {
    const e = entree.etat();
    if (e.statut !== "pret") return;
    const dernier = e.valeur.at(-1);
    if (!dernier) return;
    const selection = props.id || dernier.id;
    if (!e.valeur.some((m) => m.id === selection)) return;
    return { messages: e.valeur, dernier, selection };
  };
  onSettled(() => props.cache.consulter(props.fil));
  return (
    <Show
      when={conversation()}
      fallback={
        <Show
          when={entree.etat().statut !== "chargement"}
          fallback={<p role="status">{t("threadLoading")}</p>}
        >
          <p role="alert">
            {t("threadUnavailable")}{" "}
            <button type="button" onClick={() => void entree.charger()}>
              {t("retry")}
            </button>
          </p>
        </Show>
      }
    >
      {(courant) => (
        <>
          <h1>{courant().dernier.objet || t("noSubject")}</h1>
          <p class={styles.participants}>
            <span class={layout.sr}>{t("participants")} </span>
            {participants(courant().messages)}
          </p>
          <div class={styles.pile}>
            <For each={courant().messages} keyed={(m) => m.id}>
              {(m) => (
                <Carte
                  message={m()}
                  fil={props.fil}
                  active={courant().selection === m().id}
                  cible={props.id}
                  cache={props.cache}
                />
              )}
            </For>
          </div>
          <div class={styles.actions}>
            <For each={actions}>
              {([cle, icone]) => (
                <button type="button" disabled>
                  <Icone nom={icone} libelle={t(cle)} />
                  {t(cle)}
                </button>
              )}
            </For>
          </div>
        </>
      )}
    </Show>
  );
}
export default function EcranFil(props: { cache: Lectures }) {
  const params = useParams<{ fil: string; id?: string }>();
  const lieu = useLocation<{ retour: string }>();
  const naviguer = useNavigate();
  return (
    <main class={layout.defilement}>
      <div class={styles.colonne}>
        <button
          class={styles.retour}
          type="button"
          onClick={() =>
            lieu.state?.retour
              ? naviguer(-1)
              : naviguer("/reception", { replace: true })
          }
        >
          ← {t("back")}
        </button>
        <Show when={params.fil} keyed>
          {(fil) => <Pile fil={fil} id={params.id} cache={props.cache} />}
        </Show>
      </div>
    </main>
  );
}
