import { createEffect, onSettled } from "solid-js";
import type { VueCourrier } from "../sync";
import {
  type Appui,
  memoriserAncre,
  releverAppuis,
  restaurerPosition,
  retablirAppui,
  viserDebut,
} from "./position";

// Distance au bas de la liste, en pixels, sous laquelle la page suivante est demandée.
const APPROCHE = 600;

// Garde la place de lecture d’une liste : ancre mémorisée par vue, ligne sous
// le pointeur immobile quand la liste change, suite demandée près du bas.
// `actualisee` indique que l’actualisation de montage est partie : la liste en
// cache se peint avant toute requête, page suivante comprise.
export function suivreDefilement(
  entree: VueCourrier,
  zone: () => HTMLElement,
  actualisee: () => boolean,
) {
  let image: number | undefined;
  let pointeur: { x: number; y: number } | undefined;

  function constater() {
    const cadre = zone();
    // En haut de liste, le courrier arrivé est sous les yeux.
    if (!cadre.scrollTop) entree.arrivees.voir();
    const reste = cadre.scrollHeight - cadre.scrollTop - cadre.clientHeight;
    if (actualisee() && reste < APPROCHE && !entree.suite.echec())
      void entree.suite.charger();
  }

  function mesurer() {
    entree.repere.defilement = zone().scrollTop;
    if (image !== undefined) return;
    image = requestAnimationFrame(() => {
      image = undefined;
      memoriserAncre(zone(), entree.repere);
      constater();
    });
  }

  function ancre() {
    return entree.repere.ancre ? [entree.repere.ancre] : [];
  }

  // Un changement de largeur ou de hauteur garde la première ligne visible.
  function ajuster() {
    retablirAppui(zone(), ancre());
    mesurer();
  }

  createEffect(
    (): Appui[] => {
      entree.etat();
      // Le calcul précède l’écriture du DOM : la mesure décrit l’ancienne liste.
      const cadre = zone();
      if (!cadre) return [];
      const pointe = pointeur
        ? document.elementFromPoint?.(pointeur.x, pointeur.y)
        : undefined;
      return releverAppuis(cadre, pointe);
    },
    (appuis) => {
      const cadre = zone();
      retablirAppui(cadre, appuis);
      entree.repere.defilement = cadre.scrollTop;
      memoriserAncre(cadre, entree.repere);
      constater();
    },
    { defer: true },
  );

  onSettled(() => {
    const cadre = zone();
    restaurerPosition(cadre, entree.repere);
    // Après le retour d’un fil, l’ancre désigne la ligne réellement affichée.
    memoriserAncre(cadre, entree.repere);
    constater();
    window.addEventListener("resize", ajuster);
    const tailles =
      typeof ResizeObserver === "function"
        ? new ResizeObserver(ajuster)
        : undefined;
    for (const contenu of cadre.children) tailles?.observe(contenu);
    return () => {
      window.removeEventListener("resize", ajuster);
      tailles?.disconnect();
      if (image !== undefined) cancelAnimationFrame(image);
    };
  });

  return {
    mesurer,
    pointer(e: PointerEvent) {
      pointeur = { x: e.clientX, y: e.clientY };
    },
    quitter() {
      pointeur = undefined;
    },
    debut() {
      viserDebut(zone());
      entree.arrivees.voir();
    },
  };
}
