import { createRouter, useParams } from "@solidjs/router";
import type { JSX } from "@solidjs/web";
import { createSignal, onSettled, Show } from "solid-js";
import { Avatar } from "../components/Avatar";
import { t } from "../i18n";
import { Icone } from "../icons/Icone";
import type { Labels } from "../session/language";
import {
  type Courrier as CacheCourrier,
  creerCourrier,
  type Vue,
} from "../sync";
import { ChoixArchives } from "./Archives";
import styles from "./courrier.module.css";
import EcranFil from "./Fil";
import { Liste } from "./Liste";
import Menu from "./Menu";
import EcranTri from "./Tri";

interface Props {
  compte: number;
  adresse: string;
  textes: Labels;
  fermer: (bouton: HTMLButtonElement) => Promise<void>;
  echec: boolean;
  /** Ouvre le fil suivant après une action sur le fil ouvert. Attend son réglage. */
  lectureEnSerie?: boolean;
}

function Dossier(props: { cache: CacheCourrier }) {
  const params = useParams<{ id: string }>();
  onSettled(() => {
    void props.cache.dossiers.charger();
  });
  return (
    <Show when={Number(params.id)} keyed>
      {(id) => {
        const titre = () => {
          const dossiers = props.cache.dossiers.etat();
          if (dossiers.statut === "pret")
            return (
              dossiers.valeur.find((d) => d.id === id)?.path || t("folders")
            );
          return t("folders");
        };
        return (
          <Liste courrier={props.cache} vue={`dossier/${id}`} titre={titre()} />
        );
      }}
    </Show>
  );
}

function Barre(props: Props & { cache: CacheCourrier; children: JSX.Element }) {
  const [menu, ouvrirMenu] = createSignal(false);
  let declencheur!: HTMLButtonElement;
  let compte!: HTMLDetailsElement;
  function fermerMenu(rendreFocus: boolean) {
    ouvrirMenu(false);
    if (rendreFocus)
      queueMicrotask(() => declencheur.focus({ preventScroll: true }));
  }
  onSettled(() => {
    const raccourci = (e: KeyboardEvent) => {
      if (
        e.altKey ||
        e.ctrlKey ||
        e.metaKey ||
        e.isComposing ||
        e.key.toLowerCase() !== "h"
      )
        return;
      if (
        e.target instanceof Element &&
        e.target.closest("input,textarea,select,[contenteditable=true]")
      )
        return;
      e.preventDefault();
      if (menu()) fermerMenu(true);
      else ouvrirMenu(true);
    };
    window.addEventListener("keydown", raccourci);
    return () => window.removeEventListener("keydown", raccourci);
  });
  return (
    <div class={styles.application}>
      <header class={styles.barre}>
        <div class={styles.marque}>
          <button
            class={styles.icone}
            type="button"
            disabled
            aria-label={t("search")}
          >
            <Icone nom="recherche" libelle={t("search")} />
          </button>
          <span>Pli</span>
        </div>
        <button
          ref={declencheur}
          type="button"
          class={styles.declencheur}
          aria-label={t("menu")}
          aria-expanded={menu() ? "true" : "false"}
          aria-haspopup="dialog"
          onClick={() => ouvrirMenu(true)}
        >
          <Icone nom="menu" libelle={t("menu")} />
          {t("menu")}
        </button>
        <details ref={compte} class={styles.compte}>
          <summary aria-label={props.adresse}>
            <span aria-hidden="true">
              <Avatar nom={props.adresse} />
            </span>
            <span class={styles.adresse}>{props.adresse}</span>
          </summary>
          <div class={styles.actionsCompte}>
            <a
              href="/historique-du-tri"
              onClick={() => {
                compte.open = false;
              }}
            >
              {t("screenHistory")}
            </a>
            <button
              type="button"
              onClick={(e) => void props.fermer(e.currentTarget)}
            >
              {props.textes.close}
            </button>
            <Show when={props.echec}>
              <p role="alert">{props.textes.offline}</p>
            </Show>
          </div>
        </details>
      </header>
      {props.children}
      <Show when={menu()}>
        <Menu courrier={props.cache} fermer={fermerMenu} />
      </Show>
      <Show when={props.cache.archives.demande()}>
        {(demande) => (
          <ChoixArchives
            archives={props.cache.archives}
            lancer={demande().lancer}
          />
        )}
      </Show>
    </div>
  );
}

export default function Courrier(props: Props) {
  const cache = creerCourrier(props.compte, props.lectureEnSerie);
  const liste = (vue: Vue, titre: () => string) => () => (
    <Liste courrier={cache} vue={vue} titre={titre()} />
  );
  const Routeur = createRouter({
    routes: [
      {
        path: ["/", "/reception"],
        component: liste("reception", () => props.textes.inbox),
      },
      { path: "/flux", component: liste("flux", () => t("feed")) },
      {
        path: ["/fil/:fil", "/fil/:fil/message/:id"],
        component: () => <EcranFil courrier={cache} />,
      },
      {
        path: "/nouveaux-expediteurs",
        component: () => <EcranTri mode="attente" tri={cache.tri} />,
      },
      {
        path: "/historique-du-tri",
        component: () => <EcranTri mode="historique" tri={cache.tri} />,
      },
      {
        path: "/justificatifs",
        component: liste("justificatifs", () => t("paperTrail")),
      },
      {
        path: "/dossier/:id",
        matchFilters: { id: /^[1-9]\d*$/ },
        component: () => <Dossier cache={cache} />,
      },
      {
        path: "/ecrire",
        component: () => (
          <main class={styles.colonne}>
            <h1>{t("compose")}</h1>
            <a href="/reception">{t("inbox")}</a>
          </main>
        ),
      },
    ],
  });
  return (
    <Routeur>
      {(route) => (
        <Barre {...props} cache={cache}>
          {route.children}
        </Barre>
      )}
    </Routeur>
  );
}
