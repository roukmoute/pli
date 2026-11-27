import { useNavigate } from "@solidjs/router";
import {
  createEffect,
  For,
  Match,
  onSettled,
  Show,
  Switch,
  untrack,
} from "solid-js";
import { Avatar } from "../components/Avatar";
import { t } from "../i18n";
import { Icone } from "../icons/Icone";
import type { Courrier, Fil, Section, Vue } from "../sync";
import styles from "./courrier.module.css";
import { memoriserAncre, restaurerPosition } from "./position";

const dates = new Intl.DateTimeFormat("fr", {
  day: "numeric",
  month: "short",
  year: "numeric",
});
const reperes = [
  ["etoile", "etoile", "star"],
  ["piece_jointe", "pieceJointe", "attachment"],
  ["repondu", "repondre", "answered"],
  ["transfere", "transferer", "forwarded"],
] as const;

function Ligne(props: { fil: Fil; focus: () => void; ouvrir: () => void }) {
  const nom = () =>
    props.fil.expediteur.nom ||
    props.fil.expediteur.adresse ||
    t("unknownSender");
  const date = () => new Date(props.fil.date * 1000);
  return (
    <li>
      <button
        type="button"
        class={styles.ligne}
        data-fil={props.fil.id}
        onFocus={props.focus}
        onClick={props.ouvrir}
      >
        <span class={styles.sr}>{t(props.fil.non_lu ? "unread" : "read")}</span>
        <span class={styles.portrait} aria-hidden="true">
          <Avatar nom={nom()} />
        </span>
        <span class={styles.correspondant}>{nom()}</span>
        <span class={styles.objet}>{props.fil.objet || t("noSubject")}</span>
        <span class={styles.extrait}>{props.fil.extrait}</span>
        <span class={styles.indicateurs}>
          <For each={reperes}>
            {([champ, icone, libelle]) => (
              <Show when={props.fil[champ]}>
                <Icone nom={icone} libelle={t(libelle)} />
              </Show>
            )}
          </For>
          <span class={styles.nombre}>
            {props.fil.nombre_messages}
            <span class={styles.sr}> messages</span>
          </span>
        </span>
        <time class={styles.date} datetime={date().toISOString()}>
          {dates.format(date())}
        </time>
      </button>
    </li>
  );
}

export function Liste(props: { vue: Vue; titre: string; courrier: Courrier }) {
  const naviguer = useNavigate();
  const entree = props.courrier.obtenirVue(props.vue);
  const etat = entree.etat;
  const page = () => {
    const e = etat();
    return e.statut === "pret" ? e.valeur : undefined;
  };
  const perime = () => {
    const e = etat();
    return e.statut === "pret" && e.perime;
  };
  let zone!: HTMLElement;
  let image: number | undefined;
  createEffect(etat, (courant) => {
    if (
      courant.statut === "pret" &&
      !courant.perime &&
      courant.valeur.couverture.import_en_cours
    ) {
      const timer = window.setTimeout(
        () => void props.courrier.chargerVue(props.vue),
        1000,
      );
      return () => window.clearTimeout(timer);
    }
  });

  function memoriserDefilement() {
    entree.repere.defilement = zone.scrollTop;
    if (image !== undefined) return;
    image = requestAnimationFrame(() => {
      image = undefined;
      memoriserAncre(zone, entree.repere);
    });
  }

  function retenir(section: Section, fil: string) {
    entree.repere.section = section;
    entree.repere.fil = fil;
  }

  onSettled(() => {
    restaurerPosition(zone, entree.repere);
    window.addEventListener("resize", memoriserDefilement);
    let prochaineFrame: number | undefined;
    function actualiser() {
      prochaineFrame = undefined;
      void props.courrier.chargerVue(props.vue);
    }
    if (untrack(page)) {
      // Laisser le navigateur peindre la liste en cache avant de lancer la requête.
      prochaineFrame = requestAnimationFrame(() => {
        prochaineFrame = requestAnimationFrame(actualiser);
      });
    } else {
      actualiser();
    }
    return () => {
      window.removeEventListener("resize", memoriserDefilement);
      if (prochaineFrame !== undefined) cancelAnimationFrame(prochaineFrame);
      if (image !== undefined) cancelAnimationFrame(image);
    };
  });

  return (
    <main ref={zone} class={styles.defilement} onScroll={memoriserDefilement}>
      <div class={styles.colonne}>
        <header class={styles.enteteListe}>
          <h1>{props.titre}</h1>
          <a href="/ecrire" class={styles.ecrire} aria-label={t("compose")}>
            <Icone nom="ecrire" libelle={t("compose")} />
            {t("compose")}
          </a>
        </header>
        <Show when={props.vue === "reception" && page()}>
          {(p) => (
            <button
              type="button"
              class={styles.tri}
              onClick={() => naviguer("/nouveaux-expediteurs")}
            >
              {t("screener")} <strong>{p().expediteurs_en_attente}</strong>
            </button>
          )}
        </Show>
        <Switch>
          <Match when={page()}>
            {(contenu) => (
              <>
                <Show when={perime()}>
                  <p role="status">
                    {t("staleList")}{" "}
                    <button
                      type="button"
                      onClick={() => void props.courrier.chargerVue(props.vue)}
                    >
                      {t("retry")}
                    </button>
                  </p>
                </Show>
                <For each={["nouveaux", "deja_vus"] as const}>
                  {(section) => (
                    <section
                      class={styles.section}
                      data-section={section}
                      aria-labelledby={`titre-${section}`}
                    >
                      <h2 id={`titre-${section}`}>
                        {t(
                          section === "nouveaux" ? "newThreads" : "seenThreads",
                        )}
                      </h2>
                      <Show
                        when={contenu()[section].length}
                        fallback={
                          <p class={styles.vide}>
                            {t(
                              section === "nouveaux" ? "emptyNew" : "emptySeen",
                            )}
                          </p>
                        }
                      >
                        <ul class={styles.fils}>
                          <For
                            each={contenu()[section]}
                            keyed={(fil) => fil.id}
                          >
                            {(fil) => (
                              <Ligne
                                fil={fil()}
                                focus={() => retenir(section, fil().id)}
                                ouvrir={() => {
                                  retenir(section, fil().id);
                                  const liste = contenu()[section].map(
                                    (f) => f.id,
                                  );
                                  const suivantes = liste.splice(
                                    liste.indexOf(fil().id),
                                  );
                                  entree.repere.lecture = {
                                    fil: fil().id,
                                    section,
                                    ordre: suivantes.concat(liste.reverse()),
                                  };
                                  naviguer(
                                    `/fil/${encodeURIComponent(fil().id)}`,
                                    { state: { retour: `/${props.vue}` } },
                                  );
                                }}
                              />
                            )}
                          </For>
                        </ul>
                      </Show>
                    </section>
                  )}
                </For>
              </>
            )}
          </Match>
          <Match when={etat().statut === "erreur"}>
            <p role="alert">
              {t("listError")}{" "}
              <button
                type="button"
                onClick={() => void props.courrier.chargerVue(props.vue)}
              >
                {t("retry")}
              </button>
            </p>
          </Match>
          <Match when={etat().statut === "chargement"}>
            <p role="status">{t("listLoading")}</p>
          </Match>
        </Switch>
      </div>
    </main>
  );
}
