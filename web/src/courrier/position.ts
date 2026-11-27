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
