import { expect, it } from "vitest";
import expediteur from "../../tests/expediteur-tri.json";
import { fil, ids, page } from "../../tests/fils";
import { projeterTri } from "./projection-tri";
import type { IntentionTri } from "./tri-api";

const [f9, f7, f5] = [9, 7, 5].map((n) => fil(`f${n}`, n * 100));

function accepter(...dates: number[]): IntentionTri {
  return {
    entree: {
      ...(expediteur as IntentionTri["entree"]),
      apercus: dates.map((date) => ({
        ...fil(`admis-${date}`, date),
        expediteur: { nom: "L’Atelier", adresse: expediteur.adresse },
      })),
    },
    choix: "accepte",
    destination: "reception",
  };
}

it("admet dans la fenêtre chargée les seuls fils situés avant sa dernière ligne", () => {
  const lu = fil("lu", 600, false);
  const chargee = page([f9, f7, lu, f5], "apres-f5");
  const projetee = projeterTri(
    chargee,
    "reception",
    accepter(800, 500, 400),
    false,
  );
  expect(ids(projetee)).toEqual({
    nouveaux: ["f9", "admis-800", "f7", "f5"],
    deja_vus: ["lu"],
  });
  expect(projetee.curseur_suivant).toBe("apres-f5");
});

it("admet tous les fils d’une liste chargée jusqu’au bout ou encore vide", () => {
  const complete = projeterTri(
    page([f9, f7]),
    "reception",
    accepter(800, 400),
    false,
  );
  expect(ids(complete).nouveaux).toEqual([
    "f9",
    "admis-800",
    "f7",
    "admis-400",
  ]);
  const vide = projeterTri(
    page([], "curseur-sans-ligne"),
    "reception",
    accepter(400),
    false,
  );
  expect(ids(vide).nouveaux).toEqual(["admis-400"]);
});
