import { createSignal, Show } from "solid-js";
import { t } from "../i18n";
import styles from "./commandes.module.css";

function initiales(nom: string) {
  const mots = nom.trim().split(/\s+/u);
  const premiere = Array.from(mots[0])[0] || "";
  const derniere = mots.length > 1 ? Array.from(mots[mots.length - 1])[0] : "";
  return (premiere + derniere).toLocaleUpperCase("fr") || "?";
}

export function Avatar(props: { nom: string; src?: string }) {
  const [indisponible, setIndisponible] = createSignal<string>();
  const image = () => (props.src === indisponible() ? undefined : props.src);
  return (
    <span
      class={styles.avatar}
      role="img"
      aria-label={props.nom.trim() || t("unknownSender")}
    >
      <Show when={image()} keyed fallback={initiales(props.nom)}>
        {(src) => <img src={src} alt="" onError={() => setIndisponible(src)} />}
      </Show>
    </span>
  );
}
