import { expect, it, vi } from "vitest";
import { reponseRetenue } from "../../tests/reponse-retenue";
import { creerSelection } from "./selection";

it("réserve le lot pendant la confirmation et garde les cibles à réessayer", async () => {
  const attente = reponseRetenue();
  const fetch = vi
    .fn()
    .mockReturnValueOnce(attente.promise)
    .mockResolvedValueOnce(Response.json([{ id: "b", etat: "confirme" }]))
    .mockRejectedValueOnce(new Error("coupure"));
  vi.stubGlobal("fetch", fetch);
  const actualiser = vi.fn(async () => {});
  const choix = creerSelection("dossier/8", actualiser);
  expect(choix.echecs()).toBe(0);
  expect(choix.enCours()).toBe(false);
  choix.cocher(["a", "b", "c"], true);
  choix.cocher(["c"], false);
  await choix.marquer([], "lu", false);
  expect(fetch).not.toHaveBeenCalled();
  const ecriture = choix.marquer(["a", "b"], "lu", true);
  await choix.marquer(["a"], "etoile", true);
  expect(choix.enCours()).toBe(true);
  expect(choix.echecs()).toBe(0);
  expect(fetch).toHaveBeenCalledTimes(1);
  const [url, options] = fetch.mock.calls[0];
  expect(url).toBe("/api/v1/messages/flags");
  expect(options.method).toBe("POST");
  expect(JSON.parse(options.body)).toEqual({
    vue: "dossier/8",
    ids: ["a", "b"],
    indicateur: "lu",
    valeur: true,
  });
  attente.resolve(
    Response.json([
      { id: "a", etat: "confirme" },
      { id: "b", etat: "partiel" },
    ]),
  );
  await ecriture;
  expect(actualiser).toHaveBeenCalledWith(["a", "b"], "lu");
  expect([...choix.choisis()]).toEqual(["b"]);
  expect(choix.echecs()).toBe(1);
  expect(choix.enCours()).toBe(false);
  await choix.marquer(["b"], "etoile", false);
  expect(choix.echecs()).toBe(0);
  expect([...choix.choisis()]).toEqual(["b"]);
  await choix.marquer(["b"], "lu", false);
  expect(choix.echecs()).toBe(1);
  expect([...choix.choisis()]).toEqual(["b"]);
  expect(actualiser).toHaveBeenCalledTimes(3);
});

it("confirme plus de cent fils chargés en plusieurs requêtes", async () => {
  const lots: string[][] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url: string, options: RequestInit) => {
      const { ids } = JSON.parse(String(options.body));
      lots.push(ids);
      return Response.json(ids.map((id: string) => ({ id, etat: "confirme" })));
    }),
  );
  const choix = creerSelection(
    "reception",
    vi.fn(async () => {}),
  );
  const fils = Array.from({ length: 201 }, (_, n) => `fil-${n}`);
  choix.cocher(fils, true);
  await choix.marquer(fils, "lu", true);
  expect(lots.map((lot) => lot.length)).toEqual([100, 100, 1]);
  expect(lots.flat()).toEqual(fils);
  expect(choix.echecs()).toBe(0);
  expect(choix.choisis().size).toBe(0);
});
