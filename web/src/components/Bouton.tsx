import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";
import { Icone, type NomIcone } from "../icons/Icone";
import styles from "./commandes.module.css";

type PropsBouton = Omit<
  JSX.ButtonHTMLAttributes<HTMLButtonElement>,
  "class"
> & {
  class?: string;
};

export function Bouton(props: PropsBouton) {
  return (
    <button type="button" {...props} class={[styles.bouton, props.class]} />
  );
}

export function BoutonIcone(
  props: Omit<PropsBouton, "children" | "aria-label"> & {
    icone: NomIcone;
    libelle: string;
  },
) {
  return (
    <Bouton
      {...omit(props, "icone", "libelle")}
      data-icone
      aria-label={props.libelle}
    >
      <Icone nom={props.icone} libelle={props.libelle} />
    </Bouton>
  );
}
