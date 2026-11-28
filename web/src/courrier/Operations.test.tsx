import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { expect, it, vi } from "vitest";
import { fil, page } from "../../tests/fils";
import { reponseRetenue } from "../../tests/reponse-retenue";
import App from "../App";
import type { Fil } from "../sync";

interface Demande {
  id: string;
  type: "etoile" | "sans_etoile" | "lu" | "non_lu";
  vue: string;
  cibles: string[];
}

function effet(cible: Fil, type: Demande["type"]): Fil {
  if (type === "lu" || type === "non_lu")
    return { ...cible, non_lu: type === "non_lu" };
  return { ...cible, etoile: type === "etoile" };
}

// Le faux serveur garde l’état des fils et conclut chaque opération dès sa réponse.
function ouvrir(fils: Fil[]) {
  const serveur = {
    fils,
    revision: 2,
    lectures: 0,
    refus: [] as string[],
    retenue: Promise.resolve(),
    demandes: [] as Demande[],
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
      switch (url) {
        case "/api/v1/session":
          return Response.json({
            account_id: 3,
            address: "noa@recette.test",
            expires_at: Math.floor(Date.now() / 1000) + 300,
          });
        case "/api/v1/folders":
          return Response.json({ folders: [] });
        case "/api/v1/views/reception":
          serveur.lectures++;
          return Response.json(page(serveur.fils, null, serveur.revision));
        case "/api/v1/operations": {
          const demande: Demande = JSON.parse(String(init?.body));
          serveur.demandes.push(demande);
          await serveur.retenue;
          const confirmes = demande.cibles.filter(
            (id) => !serveur.refus.includes(id),
          );
          serveur.fils = serveur.fils.map((f) =>
            confirmes.includes(f.id) ? effet(f, demande.type) : f,
          );
          serveur.revision++;
          return Response.json({
            ...demande,
            etat: confirmes.length ? "confirmee" : "refusee",
            revision: serveur.revision,
            resultats: demande.cibles.map((id) => ({
              id,
              etat: confirmes.includes(id) ? "confirme" : "refuse",
            })),
          });
        }
        default:
          throw new Error(`Route inattendue ${url}`);
      }
    }),
  );
  window.history.replaceState(null, "", "/reception");
  const view = render(() => <App />);
  const ligne = (id: string) =>
    view.getByRole("button", { name: new RegExp(`Lettre ${id}`) });
  const rangee = (id: string) => ligne(id).parentElement as HTMLElement;
  return {
    view,
    serveur,
    ligne,
    etoile: (id: string) =>
      within(rangee(id)).getByRole("button", { name: "Étoile" }),
    avatar: (id: string) =>
      within(rangee(id)).getByRole<HTMLInputElement>("checkbox"),
    section: (id: string) => ligne(id).closest("section")?.dataset.section,
    envois: () => serveur.demandes.map(({ type, cibles }) => [type, ...cibles]),
    // La liste est relue une fois après la conclusion de chaque opération.
    async relue(fois: number) {
      await waitFor(() => expect(serveur.lectures).toBe(fois));
    },
  };
}

function retenir(serveur: ReturnType<typeof ouvrir>["serveur"]) {
  const retenue = reponseRetenue<void>();
  serveur.retenue = retenue.promise;
  return () => retenue.resolve();
}

it("étoile le fil dès le clic, signale l’attente et ne redessine que l’étoile", async () => {
  const { view, serveur, etoile, relue, envois } = ouvrir([
    fil("a", 30),
    fil("b", 20, false),
  ]);
  await view.findByRole("button", { name: /Lettre a/ });
  await relue(1);
  const bouton = etoile("a");
  expect(bouton.getAttribute("aria-pressed")).toBe("false");
  expect(bouton.dataset.attente).toBeUndefined();
  const liberer = retenir(serveur);
  const mutations: MutationRecord[] = [];
  const temoin = new MutationObserver((lot) => mutations.push(...lot));
  temoin.observe(view.getByRole("main"), {
    subtree: true,
    childList: true,
    attributes: true,
    characterData: true,
  });
  fireEvent.click(bouton);
  // La réponse est encore retenue : l’étoile a déjà changé et annonce son attente.
  await waitFor(() => expect(bouton.getAttribute("aria-pressed")).toBe("true"));
  expect(bouton.dataset.attente).toBe("oui");
  expect(envois()).toEqual([["etoile", "a"]]);
  expect(serveur.demandes[0].vue).toBe("reception");
  const description = (bouton.getAttribute("aria-describedby") as string)
    .split(" ")
    .map((id) => document.getElementById(id)?.textContent);
  expect(description).toEqual(["Lettre a", "En attente"]);
  expect(etoile("b").dataset.attente).toBeUndefined();
  for (const controle of view.getAllByRole<HTMLInputElement>("checkbox"))
    expect(controle.disabled).toBe(false);
  expect(etoile("b")).toHaveProperty("disabled", false);
  liberer();
  await waitFor(() => expect(bouton.dataset.attente).toBeUndefined());
  await relue(2);
  expect(etoile("a")).toBe(bouton);
  expect(bouton.getAttribute("aria-pressed")).toBe("true");
  expect(document.getElementById(`attente-a`)).toBeNull();
  mutations.push(...temoin.takeRecords());
  temoin.disconnect();
  expect(mutations.length).toBeGreaterThan(0);
  const ailleurs = mutations.filter(({ target }) => !bouton.contains(target));
  expect(ailleurs).toEqual([]);
});

it("marque lu puis non lu sans attendre le réseau et aboutit à non lu", async () => {
  const { view, serveur, ligne, avatar, section, relue, envois } = ouvrir([
    fil("a", 30),
    fil("b", 20, false),
  ]);
  await view.findByRole("button", { name: /Lettre a/ });
  await relue(1);
  const liberer = retenir(serveur);
  fireEvent.click(avatar("a"));
  fireEvent.click(
    await view.findByRole("button", { name: "Marquer comme lu" }),
  );
  await waitFor(() => expect(section("a")).toBe("deja_vus"));
  expect(within(ligne("a")).getByText("Lu")).toBeTruthy();
  expect(within(ligne("a")).getByText("En attente")).toBeTruthy();
  expect(within(ligne("b")).queryByText("En attente")).toBeNull();
  // Le lot a quitté la sélection : le fil se sélectionne de nouveau aussitôt.
  expect(avatar("a").checked).toBe(false);
  fireEvent.click(avatar("a"));
  fireEvent.click(
    await view.findByRole("button", { name: "Marquer comme non lu" }),
  );
  await waitFor(() => expect(section("a")).toBe("nouveaux"));
  expect(within(ligne("a")).getByText("Non lu")).toBeTruthy();
  expect(within(ligne("a")).getByText("En attente")).toBeTruthy();
  expect(envois()).toEqual([
    ["lu", "a"],
    ["non_lu", "a"],
  ]);
  expect(view.queryByRole("alert")).toBeNull();
  liberer();
  await waitFor(() =>
    expect(within(ligne("a")).queryByText("En attente")).toBeNull(),
  );
  await relue(3);
  expect(section("a")).toBe("nouveaux");
  expect(section("b")).toBe("deja_vus");
  expect(serveur.fils.map((f) => f.non_lu)).toEqual([true, false]);
  expect(view.queryByRole("alert")).toBeNull();
  expect(view.queryByRole("button", { name: "Marquer comme lu" })).toBeNull();
});

it("rétablit l’étoile refusée et l’annonce", async () => {
  const { view, serveur, etoile, relue } = ouvrir([fil("a", 30)]);
  await view.findByRole("button", { name: /Lettre a/ });
  await relue(1);
  serveur.refus = ["a"];
  const liberer = retenir(serveur);
  fireEvent.click(etoile("a"));
  await waitFor(() =>
    expect(etoile("a").getAttribute("aria-pressed")).toBe("true"),
  );
  expect(view.queryByRole("alert")).toBeNull();
  liberer();
  expect((await view.findByRole("alert")).textContent).toBe(
    "Modification non confirmée pour 1 fil.",
  );
  expect(etoile("a").getAttribute("aria-pressed")).toBe("false");
  expect(etoile("a").dataset.attente).toBeUndefined();
  await relue(2);
  expect(etoile("a").getAttribute("aria-pressed")).toBe("false");
  // Le geste suivant efface l’annonce.
  serveur.refus = [];
  fireEvent.click(etoile("a"));
  await waitFor(() => expect(view.queryByRole("alert")).toBeNull());
  await relue(3);
  expect(etoile("a").getAttribute("aria-pressed")).toBe("true");
});

it("garde sélectionnés les fils refusés d’un lot et rend le focus à la case de la page", async () => {
  const { view, serveur, avatar, section, relue, envois } = ouvrir([
    fil("a", 30),
    fil("b", 20),
    fil("c", 10, false),
  ]);
  const globale = await view.findByRole<HTMLInputElement>("checkbox", {
    name: "Sélectionner la page",
  });
  await relue(1);
  serveur.refus = ["b"];
  const liberer = retenir(serveur);
  fireEvent.click(avatar("a"));
  fireEvent.click(avatar("b"));
  await view.findByText("2 fils sélectionnés sur 3");
  const lire = view.getByRole("button", { name: "Marquer comme lu" });
  lire.focus();
  fireEvent.click(lire);
  // Les commandes du lot disparaissent avec lui : le focus rejoint la case de la page.
  expect(lire.isConnected).toBe(false);
  expect(document.activeElement).toBe(globale);
  expect(view.getByRole("status").textContent).toBe(
    "0 fils sélectionnés sur 3",
  );
  expect(envois()).toEqual([["lu", "a", "b"]]);
  // Le focus déplacé pendant l’attente reste où l’utilisateur l’a mis.
  const menu = view.getByRole("button", { name: "Menu" });
  menu.focus();
  liberer();
  expect((await view.findByRole("alert")).textContent).toBe(
    "Modification non confirmée pour 1 fil.",
  );
  await relue(2);
  expect(document.activeElement).toBe(menu);
  expect([section("a"), section("b"), section("c")]).toEqual([
    "deja_vus",
    "nouveaux",
    "deja_vus",
  ]);
  expect(avatar("a").checked).toBe(false);
  expect(avatar("b").checked).toBe(true);
  expect(globale.indeterminate).toBe(true);
  expect(window.location.pathname).toBe("/reception");
});
