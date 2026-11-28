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
import type { Suite } from "../sync/fenetre";
import type { Selection } from "../sync/selection";
import styles from "./courrier.module.css";
import { suivreDefilement } from "./defilement";
import { BarreSelection } from "./Selection";

const dates = new Intl.DateTimeFormat("fr", {
  day: "numeric",
  month: "short",
  year: "numeric",
});
const reperes = [
  ["piece_jointe", "pieceJointe", "attachment"],
  ["repondu", "repondre", "answered"],
  ["transfere", "transferer", "forwarded"],
] as const;

function Ligne(props: {
  fil: Fil;
  focus: () => void;
  ouvrir: () => void;
  selection: Selection;
}) {
  const nom = () =>
    props.fil.expediteur.nom ||
    props.fil.expediteur.adresse ||
    t("unknownSender");
  const date = () => new Date(props.fil.date * 1000);
  const coche = () => props.selection.choisis().has(props.fil.id);
  const objet = () => props.fil.objet || t("noSubject");
  return (
    <li class={styles.rangee} data-choisi={coche() ? "oui" : undefined}>
      <label class={styles.portrait}>
        <input
          type="checkbox"
          aria-label={`${t("select")} ${objet()}`}
          checked={coche()}
          onChange={(e) =>
            props.selection.cocher([props.fil.id], e.currentTarget.checked)
          }
        />
        <span aria-hidden="true">
          <Avatar nom={nom()} />
        </span>
        <span aria-hidden="true" class={styles.coche}>
          ✓
        </span>
      </label>
      <button
        type="button"
        class={styles.ligne}
        data-fil={props.fil.id}
        onFocus={props.focus}
        onClick={props.ouvrir}
      >
        <span class={styles.sr}>{t(props.fil.non_lu ? "unread" : "read")}</span>
        <span class={styles.correspondant}>{nom()}</span>
        <span id={`objet-${props.fil.id}`} class={styles.objet}>
          {objet()}
        </span>
        <span class={styles.extrait}>{props.fil.extrait}</span>
        <span class={styles.indicateurs}>
          <Show when={props.fil.attente?.lu}>
            <span class={styles.attente}>{t("pending")}</span>
          </Show>
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
      <button
        type="button"
        class={styles.etoile}
        aria-label={t("star")}
        aria-describedby={`objet-${props.fil.id} attente-${props.fil.id}`}
        aria-pressed={props.fil.etoile ? "true" : "false"}
        data-attente={props.fil.attente?.etoile ? "oui" : undefined}
        onClick={() =>
          void props.selection.marquer(
            [props.fil.id],
            "etoile",
            !props.fil.etoile,
          )
        }
      >
        <Icone nom="etoile" libelle={t("star")} />
        <Show when={props.fil.attente?.etoile}>
          <span id={`attente-${props.fil.id}`} class={styles.sr}>
            {t("pending")}
          </span>
        </Show>
      </button>
    </li>
  );
}

// Accès flottant en haut de la zone : il ne déplace aucune ligne en apparaissant.
function Nouveautes(props: { nombre: number; voir: () => void }) {
  return (
    <div class={styles.nouveautes} aria-live="polite">
      <Show when={props.nombre}>
        <button type="button" class={styles.pastille} onClick={props.voir}>
          {t("news")} <strong>{props.nombre}</strong>
        </button>
      </Show>
    </div>
  );
}

// Le défilement demande la suite de lui-même. Le bouton reste le repli au clavier.
function PageSuivante(props: { suite: Suite }) {
  return (
    <div class={styles.suite}>
      <button
        type="button"
        class={styles.suivants}
        aria-busy={props.suite.enCours() ? "true" : "false"}
        onClick={() => void props.suite.charger()}
      >
        {t("moreThreads")}
      </button>
      <Show when={props.suite.echec()}>
        <p role="alert" class={styles.echecSuite}>
          {t("moreThreadsError")}
        </p>
      </Show>
    </div>
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
  let actualisee = false;
  const defilement = suivreDefilement(
    entree,
    () => zone,
    () => actualisee,
  );
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

  function retenir(section: Section, fil: string) {
    entree.repere.section = section;
    entree.repere.fil = fil;
  }

  onSettled(() => {
    let prochaineFrame: number | undefined;
    function actualiser() {
      prochaineFrame = undefined;
      actualisee = true;
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
      if (prochaineFrame !== undefined) cancelAnimationFrame(prochaineFrame);
    };
  });

  return (
    <main
      ref={zone}
      class={styles.defilement}
      onScroll={defilement.mesurer}
      onPointerMove={defilement.pointer}
      onPointerLeave={defilement.quitter}
    >
      <div class={styles.colonne}>
        <Nouveautes nombre={entree.arrivees.nombre()} voir={defilement.debut} />
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
                <BarreSelection
                  ids={[...contenu().nouveaux, ...contenu().deja_vus].map(
                    (f) => f.id,
                  )}
                  selection={entree.selection}
                />
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
                                selection={entree.selection}
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
                <Show when={contenu().curseur_suivant}>
                  <PageSuivante suite={entree.suite} />
                </Show>
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
