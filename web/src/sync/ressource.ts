import { createSignal } from "solid-js";

export type Etat<T> =
  | { statut: "chargement" | "erreur" }
  | { statut: "pret"; valeur: T; perime: boolean };

export function ressource<T>(lire: () => Promise<T>) {
  let courant: Etat<T> = { statut: "chargement" };
  const [etat, notifier] = createSignal<Etat<T>>(courant);
  function publier(nouveau: Etat<T>) {
    courant = nouveau;
    notifier(nouveau);
  }
  let enCours: Promise<void> | undefined;
  function charger(): Promise<void> {
    if (enCours) return enCours;
    const precedent = courant;
    if (precedent.statut === "erreur") publier({ statut: "chargement" });
    const lecture = lire()
      .then(
        (valeur) => {
          if (enCours === lecture)
            publier({ statut: "pret", valeur, perime: false });
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
  function recharger() {
    enCours = undefined;
    return charger();
  }
  return { etat, charger, corriger, recharger };
}
