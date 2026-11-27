import { createSignal, For, onSettled, Show } from "solid-js";
import { Avatar } from "../components/Avatar";
import { t } from "../i18n";
import type { DestinationTri, Expediteur, ListeTri, Tri } from "../sync";
import layout from "./courrier.module.css";
import styles from "./tri.module.css";

const libelles = {
  reception: "inbox",
  flux: "feed",
  justificatifs: "paperTrail",
} as const;
const destinations = ["reception", "flux", "justificatifs"] as const;

function decision(entree: Expediteur) {
  return entree.etat === "refuse"
    ? t("screenRefused")
    : t(libelles[entree.destination]);
}

function Commandes(props: {
  entree: Expediteur;
  tri: Tri;
  avancer?: () => void;
}) {
  let choix!: HTMLSelectElement;
  return (
    <div class={styles.commandes}>
      <label>
        {t("routeTo")}
        <select ref={choix} value={props.entree.destination}>
          <For each={destinations}>
            {(d) => <option value={d}>{t(libelles[d])}</option>}
          </For>
        </select>
      </label>
      <button
        type="button"
        class={styles.admettre}
        onClick={() => {
          props.avancer?.();
          void props.tri.accepterExpediteur(
            props.entree,
            choix.value as DestinationTri,
          );
        }}
      >
        {t("accept")}
      </button>
      <button
        type="button"
        onClick={() => {
          props.avancer?.();
          void props.tri.refuserExpediteur(props.entree);
        }}
      >
        {t("refuse")}
      </button>
    </div>
  );
}

function LigneTri(props: {
  entree: Expediteur;
  tri: Tri;
  mode: ListeTri;
  charger: (operation: () => Promise<void>) => void;
}) {
  const derniere = () => props.entree.messages[0];
  const nom = () => derniere()?.nom || props.entree.adresse;
  let article!: HTMLElement;
  function avancer() {
    if (props.mode !== "attente") return;
    const titre =
      article.nextElementSibling?.querySelector("h2") ??
      article.closest("main")?.querySelector("h1");
    titre?.focus({ preventScroll: true });
  }
  return (
    <article
      ref={article}
      class={styles.expediteur}
      aria-label={props.entree.adresse}
    >
      <header class={styles.identite}>
        <Avatar nom={nom()} />
        <div>
          <h2 tabindex="-1">{nom()}</h2>
          <Show when={derniere()?.nom}>
            <p>{props.entree.adresse}</p>
          </Show>
        </div>
      </header>
      <Show when={props.mode === "historique"}>
        <p class={styles.decision}>{decision(props.entree)}</p>
      </Show>
      <Show when={derniere()}>
        {(m) => (
          <div class={styles.apercu}>
            <strong>{m().objet || t("noSubject")}</strong>
            <p>{m().extrait}</p>
          </div>
        )}
      </Show>
      <Show when={props.mode === "historique"}>
        <details class={styles.lettres}>
          <summary>{t("senderMail")}</summary>
          <For
            each={props.entree.messages}
            fallback={<p>{t("noRemainingMail")}</p>}
          >
            {(m) => (
              <p>
                <span>{m.objet || t("noSubject")}</span>
                <small>{m.extrait}</small>
              </p>
            )}
          </For>
          <Show when={props.entree.suivant}>
            <button
              type="button"
              onClick={() =>
                props.charger(() =>
                  props.tri.lettresSuivantes(props.mode, props.entree),
                )
              }
            >
              {t("moreMail")}
            </button>
          </Show>
        </details>
      </Show>
      <Show when={props.entree.revision} keyed>
        {(_revision) => (
          <Commandes entree={props.entree} tri={props.tri} avancer={avancer} />
        )}
      </Show>
    </article>
  );
}

export default function EcranTri(props: { mode: ListeTri; tri: Tri }) {
  const catalogue = props.tri[props.mode];
  type Lecture = { cle: string; operation: () => Promise<void> };
  const [erreur, signaler] = createSignal<Lecture>();
  const [lectures, publierLectures] = createSignal<string[]>([]);
  const enCours = new Set<string>();
  const contenu = () => {
    const e = catalogue.etat();
    return e.statut === "pret" ? e.valeur : undefined;
  };
  const perime = () => {
    const e = catalogue.etat();
    return e.statut === "pret" && e.perime;
  };
  const visibles = () => {
    const actifs = new Set(props.tri.intentions().map((i) => i.entree.adresse));
    return contenu()?.expediteurs.filter((e) => !actifs.has(e.adresse));
  };
  const conflit = () => {
    const avis = props.tri.avis();
    return avis?.type === "conflit" ? avis.entree : undefined;
  };
  const echec = () => {
    const avis = props.tri.avis();
    return avis?.type === "erreur" ? avis.adresse : undefined;
  };
  async function charger(cle: string, operation: () => Promise<void>) {
    if (enCours.has(cle)) return;
    enCours.add(cle);
    publierLectures([...enCours]);
    signaler(undefined);
    try {
      await operation();
    } catch {
      signaler({ cle, operation });
    } finally {
      enCours.delete(cle);
      publierLectures([...enCours]);
    }
  }
  onSettled(() => {
    void catalogue.charger();
  });
  return (
    <main class={layout.defilement}>
      <div class={layout.colonne}>
        <header class={styles.entete}>
          <h1 tabindex="-1">
            {t(props.mode === "attente" ? "screener" : "screenHistory")}
          </h1>
          <a href="/reception">{t("inbox")}</a>
        </header>
        <p class={styles.explication}>{t("screenScope")}</p>
        <Show when={conflit()}>
          {(entree) => (
            <section role="alert" class={styles.avis}>
              <p>
                {t("screenConflict")} <strong>{entree().adresse}</strong> ·{" "}
                {decision(entree())}
              </p>
              <Commandes entree={entree()} tri={props.tri} />
              <button type="button" onClick={props.tri.fermerAvis}>
                {t("closeNotice")}
              </button>
            </section>
          )}
        </Show>
        <Show when={echec()}>
          {(adresse) => (
            <p role="alert">
              {t("screenSaveError")} {adresse()}
            </p>
          )}
        </Show>
        <Show
          when={erreur() || perime() || catalogue.etat().statut === "erreur"}
        >
          <p role="alert">
            {t("screenLoadError")}{" "}
            <button
              type="button"
              onClick={() => {
                const lecture = erreur();
                if (lecture) void charger(lecture.cle, lecture.operation);
                else void charger("catalogue", catalogue.charger);
              }}
            >
              {t("retry")}
            </button>
          </p>
        </Show>
        <Show
          when={contenu()}
          fallback={
            <Show when={catalogue.etat().statut === "chargement"}>
              <p role="status">{t("screenLoading")}</p>
            </Show>
          }
        >
          <For
            each={visibles()}
            keyed={(e) => e.adresse}
            fallback={
              <p role="status" class={styles.vide}>
                {t(props.mode === "attente" ? "noScreening" : "noDecisions")}
              </p>
            }
          >
            {(entree) => (
              <LigneTri
                entree={entree()}
                mode={props.mode}
                tri={props.tri}
                charger={(f) => void charger(entree().adresse, f)}
              />
            )}
          </For>
          <Show when={contenu()?.suite}>
            <button
              type="button"
              disabled={lectures().includes("suite")}
              onClick={() =>
                void charger("suite", () => props.tri.suite(props.mode))
              }
            >
              {t("moreSenders")}
            </button>
          </Show>
        </Show>
      </div>
    </main>
  );
}
