import type { Repere, Section } from "../sync";

function lignes(zone: HTMLElement) {
  return Array.from(zone.querySelectorAll<HTMLButtonElement>("[data-fil]"));
}

function sectionDe(ligne: HTMLElement): Section {
  return ligne.closest("[data-section='deja_vus']") ? "deja_vus" : "nouveaux";
}

export function memoriserAncre(zone: HTMLElement, repere: Repere) {
  const haut = zone.getBoundingClientRect().top;
  const visible = lignes(zone).find(
    (ligne) => ligne.getBoundingClientRect().bottom > haut,
  );
  if (!visible) return;
  repere.ancre = {
    fil: visible.dataset.fil as string,
    decalage: visible.getBoundingClientRect().top - haut,
  };
  repere.section = sectionDe(visible);
}

export function restaurerPosition(zone: HTMLElement, repere: Repere) {
  zone.scrollTop = repere.defilement;
  const presentes = lignes(zone);
  function retrouver(fil: string) {
    const lecture = repere.lecture;
    if (lecture?.fil === fil) {
      const section = presentes.filter(
        (ligne) => sectionDe(ligne) === lecture.section,
      );
      for (const voisin of lecture.ordre) {
        const ligne = section.find((el) => el.dataset.fil === voisin);
        if (ligne) return ligne;
      }
    }
    return (
      presentes.find((ligne) => ligne.dataset.fil === fil) ||
      presentes.find((ligne) => sectionDe(ligne) === repere.section)
    );
  }
  if (repere.ancre) {
    const ancre = retrouver(repere.ancre.fil);
    if (ancre) {
      zone.scrollTop +=
        ancre.getBoundingClientRect().top -
        zone.getBoundingClientRect().top -
        repere.ancre.decalage;
    }
  }
  if (repere.fil) retrouver(repere.fil)?.focus({ preventScroll: true });
  delete repere.lecture;
}

export interface Appui {
  fil: string;
  decalage: number;
}

// Relevé pris avant une mise à jour de la liste : les lignes visibles et leur
// place, celle sous le pointeur en tête. En haut de liste, rien n’est retenu
// et le nouveau courrier pousse simplement les lignes.
export function releverAppuis(
  zone: HTMLElement,
  pointe: Element | null | undefined,
): Appui[] {
  if (!zone.scrollTop) return [];
  const cadre = zone.getBoundingClientRect();
  const survolee = pointe?.closest("li")?.querySelector("[data-fil]");
  const appuis: Appui[] = [];
  for (const ligne of lignes(zone)) {
    const { top, bottom } = ligne.getBoundingClientRect();
    if (bottom > cadre.top && top < cadre.bottom) {
      const appui = {
        fil: ligne.dataset.fil as string,
        decalage: top - cadre.top,
      };
      if (ligne === survolee) appuis.unshift(appui);
      else appuis.push(appui);
    }
  }
  return appuis;
}

// Replace le premier appui encore présent là où il a été relevé.
export function retablirAppui(zone: HTMLElement, appuis: Appui[]) {
  const presentes = lignes(zone);
  for (const appui of appuis) {
    const ligne = presentes.find((l) => l.dataset.fil === appui.fil);
    if (ligne) {
      zone.scrollTop +=
        ligne.getBoundingClientRect().top -
        zone.getBoundingClientRect().top -
        appui.decalage;
      return;
    }
  }
}

export function viserDebut(zone: HTMLElement) {
  zone.scrollTop = 0;
  for (const ligne of lignes(zone).slice(0, 1))
    ligne.focus({ preventScroll: true });
}
