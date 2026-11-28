import { flush } from "solid-js";
import { afterEach, expect, it, vi } from "vitest";
import catalogue from "../../tests/dossiers.json";
import { creerArchives, retenirArchives } from "./archives";
import { ressource } from "./ressource";
import type { Dossier } from "./vues";

const { inbox, atelier, classement, envoyes, archives, masque } =
  catalogue as Record<string, Dossier>;
const sansArchives = [inbox, atelier, classement, envoyes];

function ouvrir(lire: () => Promise<Dossier[]>, retenir = vi.fn()) {
  const lecture = vi.fn(lire);
  const dossiers = ressource(lecture);
  return { lecture, dossiers, choix: creerArchives(dossiers, retenir) };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

it("lance l’action sans attendre quand les archives ont leur dossier", async () => {
  const { lecture, dossiers, choix } = ouvrir(async () => [inbox, archives]);
  expect(choix.echec()).toBe(false);
  expect(choix.candidats()).toEqual([]);
  choix.preparer();
  expect(lecture).toHaveBeenCalledTimes(1);
  await dossiers.charger();
  flush();
  // Le catalogue est connu : rien n’est relu avant le geste.
  choix.preparer();
  const action = vi.fn();
  choix.avecDestination(action);
  expect(action).toHaveBeenCalledTimes(1);
  expect(lecture).toHaveBeenCalledTimes(1);
  flush();
  expect(choix.demande()).toBeUndefined();
});

it("charge le catalogue avant de décider et s’en remet au serveur s’il est illisible", async () => {
  const connu = ouvrir(async () => [inbox, archives]);
  const action = vi.fn();
  connu.choix.avecDestination(action);
  expect(action).not.toHaveBeenCalled();
  await vi.waitFor(() => expect(action).toHaveBeenCalledTimes(1));
  expect(connu.lecture).toHaveBeenCalledTimes(1);
  const illisible = ouvrir(() => Promise.reject(new Error("coupure")));
  illisible.choix.avecDestination(action);
  await vi.waitFor(() => expect(action).toHaveBeenCalledTimes(2));
  flush();
  expect(illisible.choix.demande()).toBeUndefined();
});

it("demande le dossier des archives, le retient puis lance l’action suspendue", async () => {
  const retenir = vi
    .fn<(dossier: number) => Promise<Dossier[]>>()
    .mockRejectedValueOnce(new Error("refus"))
    .mockResolvedValueOnce([inbox, { ...atelier, role: "archive" }]);
  const { dossiers, choix } = ouvrir(async () => sansArchives, retenir);
  await dossiers.charger();
  flush();
  // Seul un dossier personnel où l’on peut ranger du courrier est proposé.
  expect(choix.candidats()).toEqual([atelier]);
  const action = vi.fn();
  choix.avecDestination(action);
  flush();
  const demande = choix.demande();
  expect(demande?.lancer).toBe(action);
  expect(action).not.toHaveBeenCalled();
  expect(choix.echec()).toBe(false);

  await choix.choisir(atelier.id, action);
  flush();
  expect(retenir.mock.calls).toEqual([[atelier.id]]);
  expect(choix.echec()).toBe(true);
  expect(choix.demande()).toBe(demande);
  expect(action).not.toHaveBeenCalled();

  await choix.choisir(atelier.id, action);
  flush();
  expect(choix.echec()).toBe(false);
  expect(choix.demande()).toBeUndefined();
  expect(action).toHaveBeenCalledTimes(1);
  // Le choix est connu : le geste suivant ne demande plus rien.
  choix.avecDestination(action);
  expect(action).toHaveBeenCalledTimes(2);
});

it("oublie la demande quand l’utilisateur renonce et repart sans l’échec précédent", async () => {
  const retenir = vi.fn().mockRejectedValue(new Error("refus"));
  const { dossiers, choix } = ouvrir(async () => sansArchives, retenir);
  await dossiers.charger();
  const action = vi.fn();
  choix.avecDestination(action);
  await choix.choisir(atelier.id, action);
  flush();
  expect(choix.echec()).toBe(true);
  choix.renoncer();
  flush();
  expect(choix.demande()).toBeUndefined();
  choix.avecDestination(action);
  flush();
  expect(choix.demande()?.lancer).toBe(action);
  expect(choix.echec()).toBe(false);
  expect(action).not.toHaveBeenCalled();
});

it("enregistre le choix auprès du serveur et écarte les dossiers masqués", async () => {
  const fetch = vi.fn(async () =>
    Response.json({ folders: [masque, { ...atelier, role: "archive" }] }),
  );
  vi.stubGlobal("fetch", fetch);
  expect(await retenirArchives(atelier.id)).toEqual([
    { ...atelier, role: "archive" },
  ]);
  expect(fetch.mock.calls).toEqual([
    [
      "/api/v1/folders/archive",
      expect.objectContaining({ method: "PUT", body: '{"dossier":2}' }),
    ],
  ]);
});
