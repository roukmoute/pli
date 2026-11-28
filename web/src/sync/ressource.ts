import { createSignal } from "solid-js";

export type Etat<T> =
  | { statut: "chargement" | "erreur" }
  | { statut: "pret"; valeur: T; perime: boolean };

export function ressource<T>(
  lire: (connue?: T) => Promise<T>,
  reunir: (connue: T, lue: T) => T = (_connue, lue) => lue,
) {
  let courant: Etat<T> = { statut: "chargement" };
  const [etat, notifier] = createSignal<Etat<T>>(courant);
  function publier(nouveau: Etat<T>) {
    courant = nouveau;
    notifier(nouveau);
  }
  // Lecture directe : le signal ne rend la valeur écrite qu’après son vidage.
  function valeur() {
    return courant.statut === "pret" ? courant.valeur : undefined;
  }
  let enCours: Promise<void> | undefined;
  function charger(): Promise<void> {
    if (enCours) return enCours;
    const precedent = courant;
    if (precedent.statut === "erreur") publier({ statut: "chargement" });
    const lecture = lire(valeur())
      .then(
        (lue) => {
          if (enCours !== lecture) return;
          // La valeur a pu être complétée pendant la lecture.
          const connue = courant;
          publier({
            statut: "pret",
            valeur: connue.statut === "pret" ? reunir(connue.valeur, lue) : lue,
            perime: false,
          });
        },
        () => {
          if (enCours !== lecture) return;
          publier(
            precedent.statut === "pret"
              ? { ...precedent, perime: true }
              : { statut: "erreur" },
          );
        },
      )
      .finally(() => {
        if (enCours === lecture) enCours = undefined;
      });
    enCours = lecture;
    return lecture;
  }

  // Une réponse partie avant une écriture ne doit pas rétablir l’ancienne photographie.
  function corriger(maj: (valeur: T) => T, initial?: T) {
    const lectureSuspendue = enCours;
    enCours = undefined;
    const precedent = courant;
    if (precedent.statut === "pret")
      publier({ ...precedent, valeur: maj(precedent.valeur) });
    else if (initial !== undefined)
      publier({ statut: "pret", valeur: maj(initial), perime: false });
    else if (lectureSuspendue) void charger();
  }
  // Un complément garde la lecture en cours, qui le réunira à sa réponse.
  function completer(maj: (valeur: T) => T) {
    if (courant.statut === "pret")
      publier({ ...courant, valeur: maj(courant.valeur) });
  }
  function recharger() {
    enCours = undefined;
    return charger();
  }
  return { etat, valeur, charger, corriger, completer, recharger };
}
