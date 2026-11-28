import { flush } from "solid-js";
import { expect, it, vi } from "vitest";
import { reponseRetenue } from "../../tests/reponse-retenue";
import { creerOperations, type Intention } from "./operations";

const ID = "0190a1b2-0000-7000-8000-00000000000a";
const etoiler: Intention = {
  type: "etoile",
  vue: "reception",
  cibles: ["a", "b"],
};
const envoi = { ...etoiler, id: ID };
const coupure = new TypeError("réseau coupé");

function operation(etat: string, ...resultats: [string, string][]) {
  return Response.json({
    id: ID,
    type: "etoile",
    etat,
    revision: 7,
    resultats: resultats.map(([id, etat]) => ({ id, etat })),
  });
}
const enAttente = () =>
  operation("en_attente", ["a", "en_attente"], ["b", "en_attente"]);
const confirmee = () =>
  operation("confirmee", ["a", "confirme"], ["b", "confirme"]);
const refus = (status: number) =>
  Response.json({ code: "refus", message: "Refus." }, { status });

// Chaque appel reçoit la réponse suivante. Une erreur simule une coupure.
function serveur(...reponses: (Response | Error | Promise<Response>)[]) {
  const appels: [string, string, unknown][] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init: RequestInit) => {
      appels.push([
        String(init.method),
        url,
        init.body ? JSON.parse(String(init.body)) : undefined,
      ]);
      const suivante = reponses.shift();
      if (!suivante) throw new Error(`Appel inattendu ${url}`);
      if (suivante instanceof Error) throw suivante;
      return suivante;
    }),
  );
  return appels;
}

function preparer() {
  vi.spyOn(crypto, "randomUUID").mockReturnValue(ID);
  const pauses: number[] = [];
  const conclure = vi.fn(async () => {});
  const operations = creerOperations(conclure, async (duree) => {
    pauses.push(duree);
  });
  return { operations, conclure, pauses };
}

function file(operations: ReturnType<typeof preparer>["operations"]) {
  flush();
  return operations.enAttente();
}

const POST = ["POST", "/api/v1/operations", envoi];
const GET = ["GET", `/api/v1/operations/${ID}`, undefined];

it("montre l’intention avant toute réponse puis conclut avec les cibles confirmées", async () => {
  const retenue = reponseRetenue();
  const appels = serveur(
    retenue.promise,
    operation("confirmee", ["a", "confirme"], ["b", "refuse"]),
  );
  const { operations, conclure, pauses } = preparer();
  // La file garde l’intention jusqu’à ce que le cache ait reçu sa conclusion.
  conclure.mockImplementation(async () => {
    expect(operations.enAttente()).toEqual([envoi]);
  });
  expect(file(operations)).toEqual([]);
  const fin = operations.lancer(etoiler);
  expect(file(operations)).toEqual([envoi]);
  expect(appels).toEqual([POST]);
  retenue.resolve(enAttente());
  expect(await fin).toEqual(["b"]);
  expect(appels).toEqual([POST, GET]);
  expect(pauses).toEqual([300]);
  expect(conclure.mock.calls).toEqual([[envoi, ["a"], 7]]);
  expect(file(operations)).toEqual([]);
});

it("ne consulte pas une opération déjà conclue et garde les autres intentions en file", async () => {
  const retenue = reponseRetenue();
  const appels = serveur(retenue.promise, confirmee());
  const { operations, conclure } = preparer();
  const premiere = operations.lancer(etoiler);
  const seconde = operations.lancer({ ...etoiler, type: "sans_etoile" });
  expect(await seconde).toEqual([]);
  expect(file(operations)).toEqual([envoi]);
  retenue.resolve(operation("refusee", ["a", "refuse"], ["b", "absent"]));
  expect(await premiere).toEqual(["a", "b"]);
  expect(appels.map(([methode]) => methode)).toEqual(["POST", "POST"]);
  expect(conclure.mock.calls).toEqual([
    [{ ...envoi, type: "sans_etoile" }, ["a", "b"], 7],
    [envoi, [], 7],
  ]);
  expect(file(operations)).toEqual([]);
});

it("renvoie le même identifiant après une réponse perdue ou une panne du serveur", async () => {
  const appels = serveur(coupure, refus(500), confirmee());
  const { operations, conclure, pauses } = preparer();
  expect(await operations.lancer(etoiler)).toEqual([]);
  expect(appels).toEqual([POST, POST, POST]);
  expect(pauses).toEqual([300, 1200]);
  expect(conclure.mock.calls).toEqual([[envoi, ["a", "b"], 7]]);
});

it("abandonne après trois envois perdus et annule toutes les cibles", async () => {
  const appels = serveur(coupure, coupure, refus(503));
  const { operations, conclure, pauses } = preparer();
  expect(await operations.lancer(etoiler)).toEqual(["a", "b"]);
  expect(appels).toEqual([POST, POST, POST]);
  expect(pauses).toEqual([300, 1200]);
  expect(conclure.mock.calls).toEqual([[envoi, [], 0]]);
  expect(file(operations)).toEqual([]);
});

it("ne rejoue pas une demande que le serveur a refusée", async () => {
  for (const status of [400, 499]) {
    const appels = serveur(refus(status));
    const { operations, conclure, pauses } = preparer();
    expect(await operations.lancer(etoiler)).toEqual(["a", "b"]);
    expect(appels).toEqual([POST]);
    expect(pauses).toEqual([]);
    expect(conclure.mock.calls).toEqual([[envoi, [], 0]]);
  }
});

it("espace ses consultations et traverse une coupure pendant l’attente", async () => {
  const appels = serveur(
    enAttente(),
    enAttente(),
    coupure,
    refus(503),
    enAttente(),
    enAttente(),
    enAttente(),
    confirmee(),
  );
  const { operations, conclure, pauses } = preparer();
  expect(await operations.lancer(etoiler)).toEqual([]);
  expect(appels).toEqual([POST, GET, GET, GET, GET, GET, GET, GET]);
  expect(pauses).toEqual([300, 600, 1200, 2400, 4800, 5000, 5000]);
  expect(conclure.mock.calls).toEqual([[envoi, ["a", "b"], 7]]);
});

it("annule l’action quand l’opération devient introuvable", async () => {
  const appels = serveur(enAttente(), refus(404));
  const { operations, conclure } = preparer();
  expect(await operations.lancer(etoiler)).toEqual(["a", "b"]);
  expect(appels).toEqual([POST, GET]);
  // La révision reçue à l’envoi reste connue.
  expect(conclure.mock.calls).toEqual([[envoi, [], 7]]);
});

it("répartit plus de cent cibles en opérations de cent", async () => {
  const cibles = Array.from({ length: 201 }, (_, rang) => `fil-${rang}`);
  const lots: string[][] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url: string, init: RequestInit) => {
      const demande = JSON.parse(String(init.body));
      lots.push(demande.cibles);
      return Response.json({
        ...demande,
        etat: "confirmee",
        revision: 7,
        // Le dernier fil de chaque opération est refusé.
        resultats: demande.cibles.map((id: string, rang: number) => ({
          id,
          etat: rang + 1 === demande.cibles.length ? "refuse" : "confirme",
        })),
      });
    }),
  );
  const { operations, conclure } = preparer();
  expect(await operations.lancer({ ...etoiler, cibles })).toEqual([
    "fil-99",
    "fil-199",
    "fil-200",
  ]);
  expect(lots.map((lot) => lot.length)).toEqual([100, 100, 1]);
  expect(lots.flat()).toEqual(cibles);
  expect(conclure).toHaveBeenCalledTimes(3);
  // Deux lots pleins : aucune opération vide ne part après le dernier.
  lots.length = 0;
  await operations.lancer({ ...etoiler, cibles: cibles.slice(1) });
  expect(lots.map((lot) => lot.length)).toEqual([100, 100]);
});
