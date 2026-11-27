import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { afterAll, afterEach, beforeAll, expect, it, vi } from "vitest";
import receptionVide from "../../tests/reception-vide.json";
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
  const { vue: _vue, ...base } = receptionVide;
  return {
    ...base,
    ...liste,
    nouveaux,
    deja_vus,
    compteurs: {
      ...base.compteurs,
      reception: nouveaux.length + deja_vus.length,
    },
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

function ouvrir(
  boite: PageFils = reception,
  lireDossiers = vi.fn(() => Promise.resolve(Response.json(dossiers))),
) {
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
        return lireDossiers();
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
  return {
    view: render(() => <App />),
    lireReception,
    lireDossiers,
    fetchMock,
  };
}

afterEach(() => {
  window.history.replaceState(null, "", "/");
});

it("restaure la section, la position et la ligne pendant qu’une actualisation attend le réseau", async () => {
  window.history.replaceState(null, "", "/reception");
  const { view, lireReception, fetchMock } = ouvrir();
  const nouvelle = await view.findByRole("button", { name: /Lettre neuve/ });
  expect(nouvelle.getAttribute("data-fil")).toBe("neuve");
  expect(within(nouvelle).getByText("Non lu")).toBeTruthy();
  for (const nom of ["Étoile", "Pièce jointe", "Répondu", "Transféré"]) {
    expect(within(nouvelle).getByLabelText(nom)).toBeTruthy();
  }
  expect(nouvelle.querySelector("time")?.dateTime).toBe(
    "2025-01-01T00:00:00.000Z",
  );
  expect(within(nouvelle).getByText("Noa")).toBeTruthy();
  const ligne = await view.findByRole("button", { name: /Lettre relue/ });
  expect(within(ligne).getByText("Lu")).toBeTruthy();
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
  const atelier = await view.findByRole("button", { name: /Lettre atelier/ });
  expect(within(atelier).getByText("Non lu")).toBeTruthy();
  expect(view.queryByText("Nouveaux expéditeurs")).toBeNull();
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
  const arrivee = await view.findByRole("button", { name: /Lettre arrivée/ });
  expect(within(arrivee).getByText("Non lu")).toBeTruthy();
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
  const [nouveaux, vus] = view.getAllByRole("region");
  within(nouveaux).getByRole("heading", { name: "Nouveaux" });
  within(nouveaux).getByText("Aucun nouveau fil.");
  within(vus).getByRole("heading", { name: "Déjà vus" });
  within(vus).getByText("Aucun fil déjà vu.");
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

it("regroupe les mesures de défilement par image et annule la mesure quand la vue se ferme", async () => {
  const { view } = ouvrir();
  await view.findByRole("button", { name: /Lettre relue/ });
  const programmer = vi
    .fn<(mesurer: FrameRequestCallback) => number>()
    .mockReturnValue(41);
  const annuler = vi.fn();
  vi.stubGlobal("requestAnimationFrame", programmer);
  vi.stubGlobal("cancelAnimationFrame", annuler);
  const zone = view.getByRole("main");
  fireEvent.scroll(zone);
  fireEvent.scroll(zone);
  expect(programmer).toHaveBeenCalledTimes(1);
  programmer.mock.calls[0][0](16);
  programmer.mockReturnValue(42);
  fireEvent.scroll(zone);
  expect(programmer).toHaveBeenCalledTimes(2);
  view.unmount();
  expect(annuler).toHaveBeenCalledWith(42);
});

it("affiche le message de filtre vide uniquement quand aucun raccourci ni dossier ne correspond", async () => {
  const { view } = ouvrir();
  fireEvent.click(await view.findByRole("button", { name: "Menu" }));
  const menu = await view.findByRole("dialog");
  await within(menu).findByRole("link", { name: "Études" });
  const champ = within(menu).getByRole("searchbox");
  for (const [saisie, destination] of [
    ["envoy", "Envoyés"],
    ["justi", "Justificatifs"],
    ["études", "Études"],
  ]) {
    fireEvent.input(champ, { target: { value: saisie } });
    await within(menu).findByRole("link", { name: destination });
    expect(
      within(menu).queryByText("Aucune destination ne correspond."),
    ).toBeNull();
  }
});

it("affiche l’adresse en l’absence de nom et les libellés de remplacement d’un fil incomplet", async () => {
  const adresse = {
    ...fil("adresse", true),
    expediteur: { nom: "", adresse: "courrier@recette.test" },
  };
  const inconnu = {
    ...fil("inconnu", false),
    objet: "",
    expediteur: { nom: null, adresse: null },
  };
  const { view } = ouvrir(page({ vue: "reception" }, [adresse], [inconnu]));
  const nouveau = await view.findByRole("button", { name: /Lettre adresse/ });
  within(nouveau).getByText("courrier@recette.test");
  const ancien = view.getByRole("button", { name: /Sans objet/ });
  within(ancien).getByText("Expéditeur inconnu");
});

it("respecte les raccourcis du navigateur et restaure le focus après chacune des fermetures du menu", async () => {
  const { view } = ouvrir();
  const bouton = await view.findByRole("button", { name: "Menu" });
  const focus = vi.spyOn(bouton, "focus");
  for (const option of [
    { altKey: true },
    { ctrlKey: true },
    { metaKey: true },
    { isComposing: true },
  ]) {
    expect(fireEvent.keyDown(window, { key: "h", ...option })).toBe(true);
    expect(view.queryByRole("dialog")).toBeNull();
  }
  for (const fermeture of ["bouton", "annulation", "raccourci"]) {
    fireEvent.keyDown(window, { key: "H" });
    const menu = await view.findByRole("dialog");
    const fermer = within(menu).getByRole("button", { name: "Fermer le menu" });
    if (fermeture === "bouton") fireEvent.click(fermer);
    if (fermeture === "annulation")
      fireEvent(menu, new Event("cancel", { cancelable: true }));
    if (fermeture === "raccourci") fireEvent.keyDown(fermer, { key: "h" });
    await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    focus.mockClear();
  }
});

it("laisse le navigateur gérer les clics modifiés et garde le défilement de la fenêtre pendant une navigation", async () => {
  const { view } = ouvrir();
  const bouton = await view.findByRole("button", { name: "Menu" });
  fireEvent.click(bouton);
  const flux = await view.findByRole("link", { name: "Flux" });
  const scroll = vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const focus = vi.spyOn(bouton, "focus");
  for (const option of [
    { button: 1 },
    { ctrlKey: true },
    { metaKey: true },
    { altKey: true },
    { shiftKey: true },
  ]) {
    document.addEventListener("click", (e) => e.preventDefault(), {
      once: true,
    });
    fireEvent.click(flux, option);
    expect(view.getByRole("dialog")).toBeTruthy();
    expect(location.pathname).toBe("/");
  }
  fireEvent.click(flux);
  await view.findByRole("heading", { name: "Flux" });
  expect(scroll).not.toHaveBeenCalled();
  expect(focus).not.toHaveBeenCalled();
  scroll.mockRestore();
});

it("réessaie une liste indisponible et garde la dernière page pendant une coupure", async () => {
  const { view, lireReception } = ouvrir();
  lireReception.mockRejectedValueOnce(new TypeError("Hors ligne"));
  const alerte = await view.findByRole("alert");
  expect(alerte.textContent).toContain("Les fils sont indisponibles.");
  let repondre!: (valeur: Response) => void;
  lireReception.mockImplementationOnce(
    () =>
      new Promise((r) => {
        repondre = r;
      }),
  );
  fireEvent.click(within(alerte).getByRole("button", { name: "Réessayer" }));
  await view.findByText("Chargement des fils…");
  repondre(Response.json(reception));
  await view.findByRole("button", { name: /Lettre neuve/ });
  fireEvent.click(view.getByRole("button", { name: "Menu" }));
  fireEvent.click(await view.findByRole("link", { name: "Flux" }));
  await view.findByRole("heading", { name: "Flux" });
  lireReception.mockRejectedValueOnce(new TypeError("Coupure"));
  fireEvent.click(view.getByRole("button", { name: "Menu" }));
  fireEvent.click(await view.findByRole("link", { name: "Réception" }));
  await view.findByText("La dernière liste chargée est affichée.");
  view.getByRole("button", { name: /Lettre neuve/ });
  fireEvent.click(view.getByRole("button", { name: "Réessayer" }));
  await waitFor(() =>
    expect(
      view.queryByText("La dernière liste chargée est affichée."),
    ).toBeNull(),
  );
});

it("actualise une page en cours d’import et arrête de sonder après la fin de l’import ou une coupure", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  try {
    const partielle = {
      ...reception,
      couverture: { import_en_cours: true, proportion_indexee: 0.5 },
    };
    const { view, lireReception } = ouvrir(partielle);
    await vi.advanceTimersByTimeAsync(0);
    view.getByRole("button", { name: /Lettre neuve/ });
    expect(lireReception).toHaveBeenCalledTimes(1);
    lireReception.mockRejectedValueOnce(new Error("Coupure"));
    await vi.advanceTimersByTimeAsync(1000);
    view.getByText("La dernière liste chargée est affichée.");
    await vi.advanceTimersByTimeAsync(2000);
    expect(lireReception).toHaveBeenCalledTimes(2);
    lireReception
      .mockResolvedValueOnce(Response.json(partielle))
      .mockResolvedValueOnce(Response.json(boiteActualisee()));
    fireEvent.click(view.getByRole("button", { name: "Réessayer" }));
    await vi.advanceTimersByTimeAsync(1000);
    view.getByRole("button", { name: /Lettre arrivée/ });
    await vi.advanceTimersByTimeAsync(3000);
    expect(lireReception).toHaveBeenCalledTimes(4);
  } finally {
    vi.useRealTimers();
  }
});

it("permet de reprendre le chargement des dossiers et d’ouvrir directement un dossier absent du catalogue", async () => {
  window.history.replaceState(null, "", "/dossier/8");
  const lireDossiers = vi
    .fn()
    .mockRejectedValueOnce(new Error("Coupure"))
    .mockRejectedValueOnce(new Error("Toujours hors ligne"))
    .mockResolvedValue(Response.json({ folders: [] }));
  const { view } = ouvrir(reception, lireDossiers);
  await view.findByRole("heading", { name: "Dossiers" });
  await view.findByRole("button", { name: /Lettre atelier/ });
  fireEvent.click(view.getByRole("button", { name: "Menu" }));
  const menu = await view.findByRole("dialog");
  await within(menu).findByText("Les dossiers sont indisponibles.");
  fireEvent.click(within(menu).getByRole("button", { name: "Réessayer" }));
  await waitFor(() =>
    expect(
      within(menu).queryByText("Les dossiers sont indisponibles."),
    ).toBeNull(),
  );
  expect(view.getByRole("heading", { name: "Dossiers" })).toBeTruthy();
});
