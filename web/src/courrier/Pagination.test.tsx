import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterAll, afterEach, beforeAll, expect, it, vi } from "vitest";
import { fil, page } from "../../tests/fils";
import { reponseRetenue } from "../../tests/reponse-retenue";
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
afterEach(() => {
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, "elementFromPoint");
  window.history.replaceState(null, "", "/");
});

const HAUT = 60;
const VISIBLE = 500;
const ENTETE = 100;

// jsdom ne met rien en page : chaque ligne occupe ici une hauteur fixe, dans
// l’ordre du document, sous un en-tête de 100 pixels.
function mettreEnPage(hauteur: number) {
  const geometrie = { ligne: hauteur };
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    function (this: HTMLElement) {
      const zone = this.closest("main");
      if (!zone) return DOMRect.fromRect();
      if (this === zone) return DOMRect.fromRect({ y: HAUT, height: VISIBLE });
      const rang = Array.from(zone.querySelectorAll("[data-fil]")).indexOf(
        this,
      );
      return DOMRect.fromRect({
        y: HAUT + ENTETE + rang * geometrie.ligne - zone.scrollTop,
        height: geometrie.ligne,
      });
    },
  );
  vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(VISIBLE);
  vi.spyOn(Element.prototype, "scrollHeight", "get").mockImplementation(
    function (this: Element) {
      const lignes = this.querySelectorAll("[data-fil]").length;
      return 2 * ENTETE + lignes * geometrie.ligne;
    },
  );
  return geometrie;
}

function boite(fils: Fil[], taille: number) {
  const etat = { fils, revision: 2 };
  return {
    etat,
    lire(url: URL, portee: object): PageFils {
      const tries = [...etat.fils].sort(
        (a, b) => b.date - a.date || (a.id < b.id ? 1 : -1),
      );
      const curseur = url.searchParams.get("curseur");
      const debut = curseur ? tries.findIndex((f) => f.id === curseur) + 1 : 0;
      const fin = debut + (Number(url.searchParams.get("taille")) || taille);
      const lot = tries.slice(debut, fin);
      const suite = fin < tries.length ? lot[lot.length - 1].id : null;
      const { vue: _vue, ...reponse } = page(lot, suite, etat.revision);
      return { ...reponse, ...portee };
    },
  };
}

function ouvrir(reception: ReturnType<typeof boite>) {
  const atelier = boite(
    [9, 8, 7, 6].map((n) => fil(`a${n}`, n)),
    3,
  );
  const lireReception = vi.fn(
    async (url: URL): Promise<Response> =>
      Response.json(reception.lire(url, { vue: "reception" })),
  );
  const requetes: string[] = [];
  const fetchMock = vi.fn(
    async (adresse: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(String(adresse), "http://pli.test");
      requetes.push(url.pathname.replace("/api/v1", "") + url.search);
      switch (url.pathname) {
        case "/api/v1/session":
          return Response.json({
            account_id: 3,
            address: "noa@recette.test",
            expires_at: Math.floor(Date.now() / 1000) + 300,
          });
        case "/api/v1/views/reception":
          return lireReception(url);
        case "/api/v1/folders":
          return Response.json({
            folders: [{ id: 8, path: "Études", role: null, hidden: false }],
          });
        case "/api/v1/folders/8/messages":
          return Response.json(atelier.lire(url, { dossier: 8 }));
        case "/api/v1/operations": {
          const demande = JSON.parse(String(init?.body));
          return Response.json({
            ...demande,
            etat: "confirmee",
            revision: reception.etat.revision,
            resultats: demande.cibles.map((id: string) => ({
              id,
              etat: "confirme",
            })),
          });
        }
        default:
          throw new Error(`Route inattendue ${url}`);
      }
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  const view = render(() => <App />);
  const zone = () => view.getByRole("main");
  return {
    view,
    zone,
    lireReception,
    requetes,
    presents: () =>
      Array.from(view.container.querySelectorAll("[data-fil]")).map(
        (ligne) => (ligne as HTMLElement).dataset.fil,
      ),
    async defiler(position: number) {
      zone().scrollTop = position;
      fireEvent.scroll(zone());
      // La mesure est regroupée dans l’image suivante.
      await new Promise((suite) => requestAnimationFrame(suite));
    },
    async aller(destination: string) {
      fireEvent.click(view.getByRole("button", { name: "Menu" }));
      fireEvent.click(await view.findByRole("link", { name: destination }));
      await view.findByRole("heading", { name: destination, level: 1 });
    },
  };
}

const six = () => [6, 5, 4, 3, 2, 1].map((n) => fil(`f${n}`, n * 100));
const suivants = { name: "Afficher les fils suivants" };

it("charge la page suivante à l’approche du bas de la liste et sur demande au clavier", async () => {
  mettreEnPage(500);
  const { view, requetes, presents, defiler } = ouvrir(boite(six(), 2));
  const repli = await view.findByRole("button", suivants);
  expect(repli.getAttribute("aria-busy")).toBe("false");
  expect(presents()).toEqual(["f6", "f5"]);
  // Il reste exactement 600 pixels sous la zone visible : la suite attend.
  await defiler(100);
  expect(requetes.filter((r) => r.includes("curseur"))).toEqual([]);
  await defiler(101);
  await waitFor(() => expect(presents()).toEqual(["f6", "f5", "f4", "f3"]));
  await defiler(102);
  expect(requetes.filter((r) => r.includes("curseur"))).toEqual([
    "/views/reception?curseur=f5",
  ]);
  const attente = reponseRetenue();
  const suite = vi.mocked(fetch).getMockImplementation();
  vi.mocked(fetch).mockImplementationOnce(async (adresse) => {
    await attente.promise;
    return (suite as typeof fetch)(adresse);
  });
  fireEvent.click(repli);
  fireEvent.click(repli);
  await waitFor(() => expect(repli.getAttribute("aria-busy")).toBe("true"));
  attente.resolve(new Response());
  await waitFor(() =>
    expect(presents()).toEqual(["f6", "f5", "f4", "f3", "f2", "f1"]),
  );
  expect(view.queryByRole("button", suivants)).toBeNull();
  expect(requetes.filter((r) => r.includes("curseur"))).toEqual([
    "/views/reception?curseur=f5",
    "/views/reception?curseur=f3",
  ]);
});

it("laisse la reprise d’une suite indisponible à la commande de repli", async () => {
  mettreEnPage(500);
  const { view, lireReception, defiler, presents } = ouvrir(boite(six(), 2));
  const repli = await view.findByRole("button", suivants);
  expect(view.queryByRole("alert")).toBeNull();
  lireReception.mockRejectedValueOnce(new TypeError("Hors ligne"));
  await defiler(150);
  const alerte = await view.findByRole("alert");
  expect(alerte.textContent).toBe("Les fils suivants sont indisponibles.");
  await defiler(160);
  expect(lireReception).toHaveBeenCalledTimes(2);
  fireEvent.click(repli);
  await waitFor(() => expect(presents()).toHaveLength(4));
  expect(view.queryByRole("alert")).toBeNull();
});

it("retrouve au retour d’un dossier la ligne de la troisième page et relit toute la fenêtre", async () => {
  mettreEnPage(500);
  const { view, zone, lireReception, requetes, presents, defiler, aller } =
    ouvrir(boite(six(), 2));
  await view.findByRole("button", suivants);
  await defiler(400);
  await waitFor(() => expect(presents()).toHaveLength(4));
  await defiler(1400);
  await waitFor(() => expect(presents()).toHaveLength(6));
  const derniere = view.getByRole("button", { name: /Lettre f1/ });
  derniere.focus();
  await defiler(2600);
  await aller("Études");
  await waitFor(() => expect(presents()).toEqual(["a9", "a8", "a7"]));
  expect(requetes.at(-1)).toBe("/folders/8/messages");
  fireEvent.click(view.getByRole("button", suivants));
  await waitFor(() => expect(presents()).toEqual(["a9", "a8", "a7", "a6"]));
  expect(requetes.at(-1)).toBe("/folders/8/messages?curseur=a7");

  const attente = reponseRetenue();
  const relectures: string[] = [];
  lireReception.mockImplementation(async (url) => {
    relectures.push(url.search);
    await attente.promise;
    return Response.json(boite(six(), 2).lire(url, { vue: "reception" }));
  });
  await aller("Réception");
  const retrouvee = view.getByRole("button", { name: /Lettre f1/ });
  expect(presents()).toEqual(["f6", "f5", "f4", "f3", "f2", "f1"]);
  expect(document.activeElement).toBe(retrouvee);
  expect(zone().scrollTop).toBe(2600);
  await waitFor(() => expect(relectures).toEqual([""]));
  attente.resolve(new Response());
  await waitFor(() => expect(relectures).toEqual(["", "?curseur=f5&taille=4"]));
  await waitFor(() => expect(zone().scrollTop).toBe(2600));
  expect(presents()).toEqual(["f6", "f5", "f4", "f3", "f2", "f1"]);
  expect(document.activeElement).toBe(retrouvee);
  expect(view.queryByRole("button", { name: /Nouveautés/ })).toBeNull();
});

it("peint la liste en cache avant de demander sa page suivante", async () => {
  const geometrie = mettreEnPage(500);
  const { view, zone, requetes, presents, defiler, aller } = ouvrir(
    boite(six(), 2),
  );
  await view.findByRole("button", suivants);
  await defiler(101);
  await waitFor(() => expect(presents()).toHaveLength(4));
  await aller("Études");
  await waitFor(() => expect(presents()).toHaveLength(3));
  // Des lignes plus courtes placent le bas de la liste à portée dès le retour.
  geometrie.ligne = 100;
  const images: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (rappel: FrameRequestCallback) =>
    images.push(rappel),
  );
  const suites = () => requetes.filter((r) => r.includes("curseur"));
  const avant = ["/views/reception?curseur=f5"];
  await aller("Réception");
  expect(presents()).toEqual(["f6", "f5", "f4", "f3"]);
  fireEvent.scroll(zone());
  expect(images).toHaveLength(2);
  images[1](16);
  expect(suites()).toEqual(avant);
  // L’actualisation part à la deuxième image, puis la suite reprend.
  images[0](16);
  images[2](32);
  await waitFor(() =>
    expect(suites()).toEqual([
      ...avant,
      "/views/reception?curseur=f5&taille=2",
      "/views/reception?curseur=f3",
    ]),
  );
  await waitFor(() => expect(presents()).toHaveLength(6));
});

async function recevoir(
  reception: ReturnType<typeof boite>,
  view: ReturnType<typeof ouvrir>["view"],
  arrivee: Fil,
) {
  reception.etat.fils = [arrivee, ...reception.etat.fils];
  reception.etat.revision++;
  // L’étoile fait relire la liste, comme toute action confirmée par le serveur.
  const etoile = view.getAllByRole<HTMLButtonElement>("button", {
    name: "Étoile",
  })[0];
  fireEvent.click(etoile);
  await view.findByRole("button", { name: new RegExp(arrivee.objet) });
}

it("garde la ligne survolée en place pendant une arrivée et propose les nouveautés", async () => {
  mettreEnPage(100);
  const reception = boite(six(), 2);
  const { view, zone, presents, defiler } = ouvrir(reception);
  // Les lignes ne remplissent pas la zone : les trois pages se suivent seules.
  await waitFor(() => expect(presents()).toHaveLength(6));
  expect(view.queryByRole("button", suivants)).toBeNull();
  await defiler(250);
  const survolee = view.getByRole("button", { name: /Lettre f3/ });
  const pointer = vi.fn(() => survolee.previousElementSibling);
  Object.defineProperty(document, "elementFromPoint", {
    configurable: true,
    value: pointer,
  });
  fireEvent.pointerMove(zone(), { clientX: 30, clientY: 260 });
  expect(survolee.getBoundingClientRect().top).toBe(210);
  expect(view.queryByRole("button", { name: /Nouveautés/ })).toBeNull();

  await recevoir(reception, view, fil("g7", 700));
  const acces = await view.findByRole("button", { name: "Nouveautés 1" });
  expect(acces.parentElement?.getAttribute("aria-live")).toBe("polite");
  expect(presents()[0]).toBe("g7");
  expect(pointer).toHaveBeenCalledWith(30, 260);
  expect(survolee.getBoundingClientRect().top).toBe(210);
  expect(zone().scrollTop).toBe(350);

  // Le pointeur a quitté la liste : la première ligne visible sert d’appui.
  fireEvent.pointerLeave(zone());
  pointer.mockClear();
  await recevoir(reception, view, fil("g8", 800));
  await view.findByRole("button", { name: "Nouveautés 2" });
  expect(pointer).not.toHaveBeenCalled();
  expect(zone().scrollTop).toBe(450);

  fireEvent.click(view.getByRole("button", { name: "Nouveautés 2" }));
  expect(zone().scrollTop).toBe(0);
  expect(document.activeElement).toBe(
    view.getByRole("button", { name: /Lettre g8/ }),
  );
  await waitFor(() =>
    expect(view.queryByRole("button", { name: /Nouveautés/ })).toBeNull(),
  );
});

it("affiche en haut de liste le courrier arrivé puis l’annonce seulement plus bas", async () => {
  mettreEnPage(100);
  const reception = boite(six(), 2);
  const { view, zone, presents, defiler } = ouvrir(reception);
  await waitFor(() => expect(presents()).toHaveLength(6));
  fireEvent.pointerMove(zone(), { clientX: 30, clientY: 260 });
  await recevoir(reception, view, fil("g7", 700));
  expect(presents()[0]).toBe("g7");
  expect(zone().scrollTop).toBe(0);
  expect(view.queryByRole("button", { name: /Nouveautés/ })).toBeNull();
  await defiler(250);
  await recevoir(reception, view, fil("g8", 800));
  await view.findByRole("button", { name: "Nouveautés 1" });
  await defiler(0);
  await waitFor(() =>
    expect(view.queryByRole("button", { name: /Nouveautés/ })).toBeNull(),
  );
});

it("garde la première ligne visible quand la hauteur des lignes change", async () => {
  const geometrie = mettreEnPage(100);
  const observes: Element[] = [];
  const observateurs: { rappel: () => void; arrete: boolean }[] = [];
  vi.stubGlobal(
    "ResizeObserver",
    function Observateur(this: object, rappel: () => void) {
      const etat = { rappel, arrete: false };
      observateurs.push(etat);
      return Object.assign(this, {
        observe: (element: Element) => observes.push(element),
        disconnect: () => {
          etat.arrete = true;
        },
      });
    },
  );
  const { view, zone, presents, defiler } = ouvrir(boite(six(), 2));
  await waitFor(() => expect(presents()).toHaveLength(6));
  expect(observes).toEqual([zone().firstElementChild]);
  await defiler(250);
  // La deuxième ligne dépasse de 50 pixels en haut de la zone.
  geometrie.ligne = 140;
  observateurs[0].rappel();
  expect(zone().scrollTop).toBe(290);
  await new Promise((suite) => requestAnimationFrame(suite));
  geometrie.ligne = 180;
  window.dispatchEvent(new Event("resize"));
  expect(zone().scrollTop).toBe(330);
  expect(observateurs[0].arrete).toBe(false);
  view.unmount();
  expect(observateurs[0].arrete).toBe(true);
  geometrie.ligne = 100;
  window.dispatchEvent(new Event("resize"));
});
