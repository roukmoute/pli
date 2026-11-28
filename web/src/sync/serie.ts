import { filsDe } from "./fenetre";
import type { PageFils, Repere } from "./vues";

interface Lecture {
  repere: Repere;
  source: { valeur: () => PageFils | undefined };
}

/**
 * Lecture en série : le fil qui suit `fil` dans l’ordre de lecture de la vue devient
 * le fil ouvert. Rend son identifiant, ou rien quand la liste n’a plus de fil à ouvrir.
 */
export function avancer(entree: Lecture, fil: string): string | undefined {
  const lecture = entree.repere.lecture;
  const page = entree.source.valeur();
  if (!lecture || !page) return;
  const presents = new Set(filsDe(page).map((present) => present.id));
  const ordre = lecture.ordre.filter((id) => id !== fil && presents.has(id));
  const suivant = ordre[0];
  if (!suivant) return;
  entree.repere.lecture = { ...lecture, fil: suivant, ordre };
  entree.repere.fil = suivant;
  return suivant;
}
