import { expect, it } from "vitest";
import { fil, page } from "../../tests/fils";
import { avancer } from "./serie";
import type { PageFils, Repere } from "./vues";

function vue(valeur: PageFils | undefined, ordre?: string[]) {
  const repere: Repere = {
    defilement: 0,
    section: "nouveaux",
    ordre: "date",
    filtre: "tous",
    fil: "b",
  };
  if (ordre) repere.lecture = { fil: "b", section: "nouveaux", ordre };
  return { repere, source: { valeur: () => valeur } };
}

it("ouvre le fil suivant encore présent dans l’ordre de lecture", () => {
  // Le fil c a quitté la liste depuis l’ouverture de b.
  const reception = page([fil("a", 40), fil("b", 30), fil("d", 10)]);
  const entree = vue(reception, ["b", "c", "d", "a"]);
  expect(avancer(entree, "b")).toBe("d");
  expect(entree.repere.fil).toBe("d");
  expect(entree.repere.lecture).toEqual({
    fil: "d",
    section: "nouveaux",
    ordre: ["d", "a"],
  });
  expect(avancer(entree, "d")).toBe("a");
  expect(entree.repere.lecture?.ordre).toEqual(["a"]);
  // Dernier fil de la série : la liste reprend la main.
  expect(avancer(entree, "a")).toBeUndefined();
  expect(entree.repere.fil).toBe("a");
});

it("ne propose rien sans ordre de lecture ni liste chargée", () => {
  const reception = page([fil("a", 40), fil("b", 30)]);
  const sansOrdre = vue(reception);
  expect(avancer(sansOrdre, "b")).toBeUndefined();
  expect(sansOrdre.repere.fil).toBe("b");
  const sansListe = vue(undefined, ["b", "a"]);
  expect(avancer(sansListe, "b")).toBeUndefined();
  expect(sansListe.repere.lecture?.fil).toBe("b");
});
