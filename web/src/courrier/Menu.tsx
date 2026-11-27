import { useNavigate } from "@solidjs/router";
import { createSignal, For, onSettled, Show } from "solid-js";
import { t } from "../i18n";
import { Icone, type NomIcone } from "../icons/Icone";
import type { Courrier, Dossier } from "../sync";
import styles from "./courrier.module.css";

interface Destination {
  titre: string;
  icone: NomIcone;
  lien?: string;
}
const tuiles: Destination[] = [
  { titre: t("inbox"), icone: "reception", lien: "/reception" },
  { titre: t("feed"), icone: "flux", lien: "/flux" },
  { titre: t("paperTrail"), icone: "justificatifs", lien: "/justificatifs" },
  { titre: t("replyLater"), icone: "plusTard" },
  { titre: t("setAside"), icone: "deCote" },
  { titre: t("files"), icone: "pieceJointe" },
];
const roles = [
  ["drafts", "drafts", "ecrire"],
  ["sent", "sent", "transferer"],
  ["archive", "archives", "archive"],
  ["junk", "junk", "dossier"],
  ["trash", "trash", "corbeille"],
] as const;

function destinations(dossiers: Dossier[]): Destination[] {
  const usuels: Destination[] = roles.map(([role, label, icone]) => {
    const dossier = dossiers.find((d) => d.role === role);
    return {
      titre: t(label),
      icone,
      lien: dossier && `/dossier/${dossier.id}`,
    };
  });
  return usuels.concat(
    dossiers
      .filter((d) => d.role === null)
      .map((d) => ({
        titre: d.path,
        icone: "dossier" as const,
        lien: `/dossier/${d.id}`,
      })),
  );
}

function normaliser(texte: string) {
  return texte
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLocaleLowerCase("fr")
    .trim();
}

export default function Menu(props: {
  courrier: Courrier;
  fermer: (rendreFocus: boolean) => void;
}) {
  const navigate = useNavigate();
  const [filtre, setFiltre] = createSignal("");
  let dialogue!: HTMLDialogElement;
  let champ!: HTMLInputElement;
  const dossiers = () => {
    const etat = props.courrier.dossiers.etat();
    return etat.statut === "pret" ? etat.valeur : [];
  };
  const garder = (item: Destination) =>
    normaliser(item.titre).includes(normaliser(filtre()));
  const principales = () => tuiles.filter(garder);
  const secondaires = () => destinations(dossiers()).filter(garder);

  onSettled(() => {
    dialogue.showModal();
    champ.focus();
    void props.courrier.dossiers.charger();
  });

  function entree(item: Destination) {
    const contenu = (
      <>
        <Icone nom={item.icone} libelle={item.titre} />
        <span>{item.titre}</span>
      </>
    );
    if (!item.lien)
      return (
        <button
          type="button"
          disabled
          class={styles.destination}
          aria-label={item.titre}
        >
          {contenu}
        </button>
      );
    const lien = item.lien;
    return (
      <a
        class={styles.destination}
        href={lien}
        aria-label={item.titre}
        onClick={(e) => {
          if (
            e.button !== 0 ||
            e.ctrlKey ||
            e.metaKey ||
            e.altKey ||
            e.shiftKey
          )
            return;
          e.preventDefault();
          props.fermer(false);
          navigate(lien, { scroll: false });
        }}
      >
        {contenu}
      </a>
    );
  }

  return (
    <dialog
      ref={dialogue}
      class={styles.menu}
      aria-label={t("menu")}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          props.fermer(true);
        }
      }}
      onCancel={(e) => {
        e.preventDefault();
        props.fermer(true);
      }}
    >
      <div class={styles.rechercheMenu}>
        <Icone nom="recherche" libelle={t("destination")} />
        <input
          ref={champ}
          type="search"
          aria-label={t("destination")}
          placeholder={t("destination")}
          value={filtre()}
          onInput={(e) => setFiltre(e.currentTarget.value)}
        />
        <button
          type="button"
          aria-label={t("closeMenu")}
          onClick={() => props.fermer(true)}
        >
          <Icone nom="fermer" libelle={t("closeMenu")} />
        </button>
      </div>
      <div class={styles.tuiles}>
        <For each={principales()}>{entree}</For>
      </div>
      <div class={styles.dossiers}>
        <For each={secondaires()}>{entree}</For>
      </div>
      <Show when={!principales().length && !secondaires().length}>
        <p role="status">{t("emptyFilter")}</p>
      </Show>
      <Show when={props.courrier.dossiers.etat().statut === "erreur"}>
        <p role="status">
          {t("foldersError")}{" "}
          <button
            type="button"
            onClick={() => void props.courrier.dossiers.charger()}
          >
            {t("retry")}
          </button>
        </p>
      </Show>
    </dialog>
  );
}
