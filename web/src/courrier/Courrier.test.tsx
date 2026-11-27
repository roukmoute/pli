import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { afterAll, afterEach, beforeAll, expect, it, vi } from "vitest";
import App from "../App";
import type { Fil, PageFils } from "../sync";

// jsdom fournit HTMLDialogElement sans l’ouverture modale native du navigateur.
const prototypeDialogue = HTMLDialogElement.prototype;
const ouvertureNative = Object.getOwnPropertyDescriptor(
  prototypeDialogue,
  "showModal",
);
beforeAll(() => {
  if (!ouvertureNative)
    Object.defineProperty(prototypeDialogue, "showModal", {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.open = true;
      },
    });
});
afterAll(() => {
  if (!ouvertureNative) Reflect.deleteProperty(prototypeDialogue, "showModal");
});

function fil(id: string, nonLu: boolean): Fil {
  return {
    id,
    objet: `Lettre ${id}`,
    expediteur: { nom: "Noa", adresse: "noa@recette.test" },
    extrait: "Extrait de la lettre",
    date: 1735689600,
    nombre_messages: 2,
    non_lu: nonLu,
    etoile: true,
    piece_jointe: true,
    repondu: true,
    transfere: true,
  };
}

function page(
  liste: { vue: string } | { dossier: number },
  nouveaux: Fil[],
  deja_vus: Fil[],
): PageFils {
  return {
    ...liste,
    compte: 3,
    nouveaux,
    deja_vus,
    revision: 2,
    expediteurs_en_attente: 4,
    curseur_suivant: null,
    couverture: { import_en_cours: false, proportion_indexee: 1 },
    compteurs: { reception: 2, flux: 0, justificatifs: 0 },
  };
}

const reception = page(
  { vue: "reception" },
  [fil("neuve", true)],
  [fil("relue", false)],
);
const dossiers = {
  folders: [
    { id: 8, path: "Études", role: null, hidden: false },
    { id: 9, path: "Masqué", role: null, hidden: true },
    { id: 7, path: "Sent", role: "sent", hidden: false },
  ],
};

function ouvrir(boite: PageFils = reception) {
  const session = {
    account_id: 3,
    address: "noa@recette.test",
    expires_at: Math.floor(Date.now() / 1000) + 300,
  };
  const lireReception = vi.fn(
    (): Promise<Response> => Promise.resolve(Response.json(boite)),
  );
  const fetchMock = vi.fn(async (url: RequestInfo | URL) => {
    switch (url) {
      case "/api/v1/session":
        return Response.json(session);
      case "/api/v1/views/reception":
        return lireReception();
      case "/api/v1/folders":
        return Response.json(dossiers);
      case "/api/v1/views/flux":
        return Response.json(page({ vue: "flux" }, [], []));
      case "/api/v1/views/justificatifs":
        return Response.json(page({ vue: "justificatifs" }, [], []));
      case "/api/v1/folders/8/messages":
        return Response.json(page({ dossier: 8 }, [fil("atelier", true)], []));
      default:
        throw new Error(`Route inattendue ${url}`);
    }
  });
  vi.stubGlobal("fetch", fetchMock);
  return { view: render(() => <App />), lireReception, fetchMock };
}

afterEach(() => {
  window.history.replaceState(null, "", "/");
});

it("restaure la section, la position et la ligne pendant qu’une actualisation attend le réseau", async () => {
  window.history.replaceState(null, "", "/reception");
  const { view, lireReception, fetchMock } = ouvrir();
  const ligne = await view.findByRole("button", { name: /Lettre relue/ });
  expect(ligne.closest("section")?.dataset.section).toBe("deja_vus");
  ligne.focus();
  const liste = view.getByRole("main");
  liste.scrollTop = 164;
  fireEvent.scroll(liste);
  fireEvent.click(view.getByRole("button", { name: "Menu" }));
  const menu = await view.findByRole("dialog");
  const champ = within(menu).getByRole("searchbox");
  expect(document.activeElement).toBe(champ);
  await within(menu).findByRole("link", { name: "Études" });
  expect(within(menu).queryByText("Masqué")).toBeNull();
  fireEvent.input(champ, { target: { value: "etudes" } });
  await waitFor(() =>
    expect(within(menu).queryByRole("link", { name: "Réception" })).toBeNull(),
  );
  fireEvent.click(within(menu).getByRole("link", { name: "Études" }));
  await view.findByRole("button", { name: /Lettre atelier/ });
  expect(location.pathname).toBe("/dossier/8");
  expect(view.getByRole("heading", { name: "Études" })).toBeTruthy();
  let repondre!: (valeur: Response) => void;
  lireReception.mockImplementationOnce(
    () =>
      new Promise((r) => {
        repondre = r;
      }),
  );
  fireEvent.click(view.getByRole("button", { name: "Menu" }));
  fireEvent.click(await view.findByRole("link", { name: "Réception" }));
  const retrouvee = await view.findByRole("button", { name: /Lettre relue/ });
  expect(location.pathname).toBe("/reception");
  expect(lireReception).toHaveBeenCalledTimes(2);
  expect(document.activeElement).toBe(retrouvee);
  expect(retrouvee.closest("section")?.dataset.section).toBe("deja_vus");
  expect(view.getByRole("main").scrollTop).toBe(164);
  const largeur = window.innerWidth;
  vi.stubGlobal("innerWidth", 390);
  window.dispatchEvent(new Event("resize"));
  expect(document.activeElement).toBe(retrouvee);
  expect(view.getByRole("main").scrollTop).toBe(164);
  vi.stubGlobal("innerWidth", largeur);
  repondre(Response.json({ ...boiteActualisee(), revision: 3 }));
  await view.findByRole("button", { name: /Lettre arrivée/ });
  expect(document.activeElement).toBe(retrouvee);
  expect(
    fetchMock.mock.calls.some(([url]) => url === "/api/v1/folders/8/messages"),
  ).toBe(true);
});

function boiteActualisee() {
  return {
    ...reception,
    nouveaux: [fil("arrivée", true), ...reception.nouveaux],
  };
}

it("garde le compteur du tri en réception vide et rend le focus au menu", async () => {
  const { view } = ouvrir(page({ vue: "reception" }, [], []));
  await view.findByRole("button", { name: /Nouveaux expéditeurs 4/ });
  view.getByText("Aucun nouveau fil.");
  view.getByText("Aucun fil déjà vu.");
  fireEvent.keyDown(window, { key: "h" });
  const menu = await view.findByRole("dialog", { name: "Menu" });
  expect(within(menu).getAllByRole("link").length).toBeGreaterThan(0);
  for (const label of [
    "Réception",
    "Flux",
    "Justificatifs",
    "Répondre plus tard",
    "De côté",
    "Fichiers",
    "Brouillons",
    "Envoyés",
    "Archives",
    "Indésirables",
    "Corbeille",
  ]) {
    expect(
      within(menu).getByLabelText(label, { selector: "a,button" }),
    ).toBeTruthy();
  }
  const champ = within(menu).getByRole("searchbox");
  fireEvent.input(champ, { target: { value: "aucun ici" } });
  expect(
    await within(menu).findByText("Aucune destination ne correspond."),
  ).toBeTruthy();
  fireEvent.keyDown(champ, { key: "h" });
  expect(view.queryByRole("dialog")).toBe(menu);
  fireEvent.keyDown(champ, { key: "Escape" });
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(document.activeElement).toBe(
    view.getByRole("button", { name: "Menu" }),
  );
  for (const svg of view.container.querySelectorAll("svg"))
    expect(svg.getAttribute("aria-label")).toBeTruthy();
});

it("ouvre les vues logiques et la route Écrire depuis les commandes de navigation", async () => {
  const { view } = ouvrir();
  for (const titre of ["Flux", "Justificatifs", "Réception"]) {
    fireEvent.click(await view.findByRole("button", { name: "Menu" }));
    fireEvent.click(await view.findByRole("link", { name: titre }));
    await view.findByRole("heading", { name: titre, level: 1 });
  }
  fireEvent.click(view.getByRole("link", { name: "Écrire" }));
  await view.findByRole("heading", { name: "Écrire" });
  expect(location.pathname).toBe("/ecrire");
});
