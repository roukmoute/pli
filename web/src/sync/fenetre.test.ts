import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import { fil, ids, page } from "../../tests/fils";
import { reponseRetenue } from "../../tests/reponse-retenue";
import {
  creerArrivees,
  creerSuite,
  etendre,
  fusionnerFils,
  integrer,
  type LecteurDePage,
  relire,
} from "./fenetre";
import type { Etat } from "./ressource";
import type { PageFils } from "./vues";

const [f9, f8, f7, f6, f5, f4] = [9, 8, 7, 6, 5, 4].map((n) =>
  fil(`f${n}`, n * 100),
);

function lecteur(...reponses: PageFils[]) {
  const lire = vi.fn<LecteurDePage>();
  for (const reponse of reponses) lire.mockResolvedValueOnce(reponse);
  return lire;
}

it("fusionne deux pages aux dates identiques sans doublon, dans l’ordre des identifiants", () => {
  const memeDate = ["b", "d", "a", "c"].map((id) => fil(id, 500));
  const relu = { ...memeDate[2], non_lu: false };
  const fusion = fusionnerFils(
    [memeDate[0], memeDate[1], fil("recent", 501)],
    [memeDate[2], memeDate[3], memeDate[1]],
    [relu],
  );
  expect(fusion.map((f) => f.id)).toEqual(["recent", "d", "c", "b", "a"]);
  expect(fusion[4]).toBe(relu);
  expect(fusionnerFils()).toEqual([]);
});

it("ajoute la page suivante dans les deux sections et garde la révision la plus ancienne", () => {
  const lu = fil("lu", 650, false);
  const ancien = fil("ancien", 350, false);
  const premiere = page([f9, f8, lu], "apres-lu", 7);
  const suite = { ...page([f6, ancien], "apres-ancien", 5), compte: 12 };
  const fenetre = etendre(premiere, suite);
  expect(ids(fenetre)).toEqual({
    nouveaux: ["f9", "f8", "f6"],
    deja_vus: ["lu", "ancien"],
  });
  expect(fenetre).toMatchObject({
    curseur_suivant: "apres-ancien",
    revision: 5,
    compte: 12,
  });
  expect(etendre(suite, premiere).revision).toBe(5);
});

it("remplace la fenêtre par une relecture qui la couvre ou qui atteint la fin de la boîte", () => {
  const fenetre = page([f9, f8, f7], "apres-f7");
  const couvrante = page([f9, f8, f7], "autre-curseur", 3);
  expect(integrer(fenetre, couvrante)).toBe(couvrante);
  // Sans curseur, la boîte s’arrête là : les lignes absentes ont disparu.
  const raccourcie = page([f9]);
  expect(integrer(fenetre, raccourcie)).toBe(raccourcie);
});

it("garde les pages chargées au-delà d’une relecture partie plus tôt", () => {
  const remonte = { ...f5, date: 950, objet: "Réponse reçue" };
  const lu = fil("lu", 450, false);
  const fenetre = page([f9, f8, f7, f6, f5, lu, f4], "apres-f4", 2);
  const frais = { ...page([remonte, f9, f8], "apres-f8", 6), compte: 12 };
  const reunie = integrer(fenetre, frais);
  expect(ids(reunie)).toEqual({
    nouveaux: ["f5", "f9", "f8", "f7", "f6", "f4"],
    deja_vus: ["lu"],
  });
  expect(reunie.nouveaux[0]).toBe(remonte);
  expect(reunie).toMatchObject({
    curseur_suivant: "apres-f4",
    revision: 6,
    compte: 12,
  });
});

it("lit une première page sans rien demander de plus", async () => {
  const premiere = page([f9, f8], "apres-f8");
  const lire = lecteur(premiere);
  expect(await relire(lire)).toBe(premiere);
  expect(lire.mock.calls).toEqual([[]]);
});

it("relit toutes les pages connues en demandant exactement les lignes qui restent", async () => {
  const connue = page([f9, f8, f7, f6, f5], "apres-f5");
  const lire = lecteur(
    page([f9, f8], "apres-f8"),
    page([f7, f6, f5], "apres-f5"),
  );
  const relue = await relire(lire, connue);
  expect(ids(relue).nouveaux).toEqual(["f9", "f8", "f7", "f6", "f5"]);
  expect(relue.curseur_suivant).toBe("apres-f5");
  expect(lire.mock.calls).toEqual([[], [{ curseur: "apres-f8", taille: 3 }]]);
});

it("réunit deux pages successives de fils aux dates identiques en un ensemble complet", async () => {
  const [d, c, b, a] = ["d", "c", "b", "a"].map((id) => fil(id, 500));
  const lire = lecteur(page([d, c], "apres-c"), page([b, a], "apres-a"));
  const relue = await relire(lire, page([d, c, b, a], "apres-a"));
  expect(ids(relue).nouveaux).toEqual(["d", "c", "b", "a"]);
  expect(lire.mock.calls).toEqual([[], [{ curseur: "apres-c", taille: 2 }]]);
  // Seul l’identifiant place « a » après la relecture : sa page reste chargée.
  const reunie = integrer(relue, page([d, c, b], "apres-b"));
  expect(ids(reunie).nouveaux).toEqual(["d", "c", "b", "a"]);
  expect(reunie.curseur_suivant).toBe("apres-a");
});

it("poursuit avec la taille réglée quand une arrivée a repoussé la dernière ligne connue", async () => {
  const arrivee = fil("arrivee", 1000);
  const connue = page([f9, f8, f7, f6, f5], "apres-f5");
  const lire = lecteur(
    page([arrivee, f9], "apres-f9"),
    page([f8, f7, f6], "apres-f6"),
    page([f5, f4], "apres-f4"),
  );
  const relue = await relire(lire, connue);
  expect(ids(relue).nouveaux).toEqual([
    "arrivee",
    "f9",
    "f8",
    "f7",
    "f6",
    "f5",
    "f4",
  ]);
  expect(relue.curseur_suivant).toBe("apres-f4");
  expect(lire.mock.calls).toEqual([
    [],
    [{ curseur: "apres-f9", taille: 3 }],
    [{ curseur: "apres-f6", taille: 0 }],
  ]);
});

it("plafonne une requête de relecture à cent lignes et s’arrête à la fin de la boîte", async () => {
  const connus = Array.from({ length: 150 }, (_, n) => fil(`c${n}`, 5000 - n));
  const lire = lecteur(
    page(connus.slice(0, 20), "apres-20"),
    page(connus.slice(20, 120), "apres-120"),
    // La dernière ligne connue a été supprimée : la boîte se termine avant elle.
    page(connus.slice(120, 149)),
  );
  const relue = await relire(lire, page(connus, "apres-150"));
  expect(relue.nouveaux).toHaveLength(149);
  expect(relue.curseur_suivant).toBeNull();
  expect(lire.mock.calls).toEqual([
    [],
    [{ curseur: "apres-20", taille: 100 }],
    [{ curseur: "apres-120", taille: 30 }],
  ]);
});

it("relit une seule fois quand la boîte a changé entre deux requêtes", async () => {
  const connue = page([f9, f8, f7], "apres-f7");
  const remonte = { ...f7, date: 990 };
  const lire = lecteur(
    page([f9, f8], "apres-f8", 4),
    page([f6], null, 5),
    page([remonte, f9], "apres-f9", 5),
    page([f8, f6], null, 5),
  );
  const relue = await relire(lire, connue);
  expect(ids(relue).nouveaux).toEqual(["f7", "f9", "f8", "f6"]);
  expect(relue.revision).toBe(5);
  expect(lire).toHaveBeenCalledTimes(4);

  const agitee = lecteur(
    page([f9, f8], "apres-f8", 6),
    page([f7], null, 7),
    page([f9, f8], "apres-f8", 8),
    page([f7], null, 9),
  );
  expect((await relire(agitee, connue)).revision).toBe(8);
  expect(agitee).toHaveBeenCalledTimes(4);
});

function source(fenetre?: PageFils) {
  let valeur = fenetre;
  return {
    valeur: () => valeur,
    completer: vi.fn((maj: (actuelle: PageFils) => PageFils) => {
      valeur = maj(valeur as PageFils);
    }),
    charger: vi.fn(() => Promise.resolve()),
  };
}

it("ne demande aucune suite avant la première page ni après la dernière", async () => {
  const lire = lecteur();
  await creerSuite(source(), lire).charger();
  await creerSuite(source(page([f9])), lire).charger();
  expect(lire).not.toHaveBeenCalled();
});

it("charge la page suivante une seule fois et l’ajoute à la fenêtre du moment", async () => {
  const depot = source(page([f9, f8], "apres-f8"));
  const attente = reponseRetenue<PageFils>();
  const lire = vi.fn<LecteurDePage>(() => attente.promise);
  const suite = creerSuite(depot, lire);
  expect(suite.enCours()).toBe(false);
  expect(suite.echec()).toBe(false);
  const demande = suite.charger();
  expect(suite.charger()).toBe(demande);
  flush();
  expect(suite.enCours()).toBe(true);
  expect(lire.mock.calls).toEqual([[{ curseur: "apres-f8" }]]);
  attente.resolve(page([f7, f6], "apres-f6"));
  await demande;
  flush();
  expect(suite.enCours()).toBe(false);
  expect(suite.echec()).toBe(false);
  expect(ids(depot.valeur() as PageFils).nouveaux).toEqual([
    "f9",
    "f8",
    "f7",
    "f6",
  ]);
  expect(depot.charger).not.toHaveBeenCalled();
  expect(suite.charger()).not.toBe(demande);
});

it("relit le haut de la liste quand la page suivante vient d’une boîte modifiée", async () => {
  const depot = source(page([f9, f8], "apres-f8", 2));
  const suite = creerSuite(depot, lecteur(page([f7], null, 3)));
  await suite.charger();
  expect(depot.completer).toHaveBeenCalledTimes(1);
  expect(depot.charger).toHaveBeenCalledTimes(1);
});

it("signale une suite indisponible puis efface l’échec à la reprise", async () => {
  const depot = source(page([f9], "apres-f9"));
  const lire = vi
    .fn<LecteurDePage>()
    .mockRejectedValueOnce(new Error("Hors ligne"))
    .mockResolvedValueOnce(page([f8]));
  const suite = creerSuite(depot, lire);
  await suite.charger();
  flush();
  expect(suite.echec()).toBe(true);
  expect(suite.enCours()).toBe(false);
  expect(depot.completer).not.toHaveBeenCalled();
  const reprise = suite.charger();
  flush();
  expect(suite.echec()).toBe(false);
  await reprise;
  expect(ids(depot.valeur() as PageFils).nouveaux).toEqual(["f9", "f8"]);
});

it("compte les fils passés devant le dernier courrier vu en haut de liste", () => {
  let etat: Etat<PageFils> = { statut: "chargement" };
  const arrivees = creerArrivees(() => etat);
  arrivees.voir();
  flush();
  expect(arrivees.nombre()).toBe(0);
  const lu = fil("lu", 850, false);
  etat = { statut: "pret", valeur: page([f8, lu, f7]), perime: false };
  // Rien n’a encore été vu : la première liste n’annonce aucune arrivée.
  expect(arrivees.nombre()).toBe(0);
  arrivees.voir();
  flush();
  expect(arrivees.nombre()).toBe(0);
  const reponse = { ...f7, date: 990 };
  etat = {
    statut: "pret",
    valeur: page([reponse, f9, lu, f8]),
    perime: false,
  };
  expect(arrivees.nombre()).toBe(2);
  arrivees.voir();
  flush();
  expect(arrivees.nombre()).toBe(0);
});
