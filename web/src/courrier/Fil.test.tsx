import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import reception from "../../tests/reception-vide.json";
import { reponseRetenue } from "../../tests/reponse-retenue";
import App from "../App";
import type { Contenu, Entete } from "../sync/lecture";

function entete(
  id: string,
  nom: string | null,
  objet: string,
  non_lu = false,
): Entete {
  return {
    id,
    nom,
    objet,
    adresse: `${nom?.toLowerCase()}@recette.test`,
    date: 1735689600,
    destinataires: {},
    non_lu,
  };
}
const plan = "1/plan",
  lettre = "2/jardin";
const lea = entete("2", "Léa", "Plan revu", true);
lea.destinataires = {
  to: [
    { name: "Zoé", address: "zoe@recette.test" },
    { name: "Doublon", address: lea.adresse as string },
  ],
  cc: [{ name: null, address: "copie@recette.test" }],
};
function corps(texte: string, extra: Partial<Contenu> = {}) {
  return Response.json({
    etat: "disponible",
    version: 1,
    contenu: { html: `<p>${texte}</p>`, images: [], pieces: [], ...extra },
  });
}
type Repondre = (
  path: string,
  options?: RequestInit,
) => Promise<Response> | Response;
function afficher(route: string, repondre: Repondre) {
  history.replaceState(null, "", route);
  const fetch = vi.fn(async (url: string, options?: RequestInit) => {
    if (url === "/api/v1/session")
      return Response.json({
        account_id: 3,
        address: "noa@recette.test",
        expires_at: Math.floor(Date.now() / 1000) + 600,
      });
    return repondre(url.replace("/api/v1", ""), options);
  });
  vi.stubGlobal("fetch", fetch);
  return { vue: render(() => <App />), fetch };
}
function rejoindre(route: string) {
  history.pushState(null, "", route);
  window.dispatchEvent(new PopStateEvent("popstate"));
}
const racine = (id: string) => `/fil/${encodeURIComponent(id)}`;
afterEach(() => {
  history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});

it("empile les messages, garde les citations et limite les images distantes au message ouvert", async () => {
  const ancien = { ...entete("1", null, "Premier plan"), adresse: null };
  const { vue, fetch } = afficher(racine(plan), (chemin, options) => {
    if (chemin === "/threads/1%2Fplan") return Response.json([ancien, lea]);
    if (chemin === "/messages/1") return corps("Premier courrier");
    if (chemin === "/messages/2/open") {
      expect(options?.method).toBe("POST");
      return Response.json({ fil: plan, revision: 5, non_lus: {} });
    }
    if (chemin === "/messages/2/images/7")
      return new Response(new Uint8Array([1, 2, 3]), {
        headers: { "content-type": "image/png" },
      });
    if (chemin === "/messages/2/images/8")
      return new Response(null, { status: 404 });
    if (chemin === "/messages/2")
      return corps("", {
        html: '<blockquote>Été à 京都</blockquote><img src="cid:pli-0"><img src="cid:pli-1"><img src="cid:pli-2">',
        images: [
          { type: "integree", partie: 7 },
          { type: "distante", url: "https://images.recette.test/jardin" },
          { type: "integree", partie: 8 },
        ],
        pieces: [
          { nom: "reçu.pdf", mime: "application/pdf", taille: 142 },
          { nom: null, mime: "text/plain", taille: 3 },
        ],
      });
    throw Error(chemin);
  });
  await vue.findByRole("heading", { name: "Plan revu" });
  await vue.findByText(/Léa, Zoé, copie@recette.test/);
  const cartes = vue.getAllByRole("article");
  expect(
    within(cartes[0])
      .getByRole("button", { name: "Expéditeur inconnu" })
      .getAttribute("aria-expanded"),
  ).toBe("false");
  expect(
    within(cartes[1])
      .getByRole("button", { name: "Léa" })
      .getAttribute("aria-expanded"),
  ).toBe("true");
  const heure = cartes[1].querySelector("time") as HTMLTimeElement;
  expect(heure.dateTime).toBe("2025-01-01T00:00:00.000Z");
  expect(heure.textContent).toBe(
    new Intl.DateTimeFormat("fr", {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(1735689600000),
  );
  const iframe = () =>
    vue.getByTitle("Contenu du message") as HTMLIFrameElement;
  await waitFor(() =>
    expect(iframe().srcdoc).toContain("data:image/png;base64,AQID"),
  );
  expect(iframe().getAttribute("sandbox")).toBe("");
  expect(iframe().srcdoc).toContain("<blockquote>Été à 京都</blockquote>");
  expect(iframe().srcdoc).not.toContain("https://images");
  expect(iframe().srcdoc).toContain("img-src data:;");
  expect(vue.getByText("reçu.pdf")).toBeTruthy();
  expect(vue.getByText("Pièce jointe")).toBeTruthy();
  fireEvent.click(
    vue.getByRole("button", { name: /Afficher les images distantes/ }),
  );
  await waitFor(() =>
    expect(iframe().srcdoc).toContain(
      'src="https://images.recette.test/jardin"',
    ),
  );
  expect(vue.queryByRole("button", { name: /Afficher les images/ })).toBeNull();
  const actuelle = within(cartes[1]).getByRole("button", { name: "Léa" });
  fireEvent.click(actuelle);
  await waitFor(() =>
    expect(vue.queryByTitle("Contenu du message")).toBeNull(),
  );
  fireEvent.click(
    within(cartes[0]).getByRole("button", { name: "Expéditeur inconnu" }),
  );
  await waitFor(() => expect(iframe().srcdoc).toContain("Premier courrier"));
  expect(vue.queryByRole("button", { name: /Afficher les images/ })).toBeNull();
  expect(iframe().srcdoc).not.toContain("https://images");
  expect(fetch.mock.calls.filter(([c]) => c.endsWith("/open"))).toHaveLength(1);
});

it("revient immédiatement en Réception et ignore le corps tardif du premier fil", async () => {
  const tardif = reponseRetenue();
  const actualisation = reponseRetenue();
  const fils = [plan, lettre].map((id, i) => ({
    id,
    objet: id,
    expediteur: { nom: "Noa" },
    date: 100 + i,
    non_lu: false,
    nombre_messages: 1,
    extrait: "",
  }));
  let lectures = 0;
  const { vue, fetch } = afficher("/reception", (chemin) => {
    if (chemin === "/views/reception")
      return ++lectures === 1
        ? Response.json({ ...reception, nouveaux: [], deja_vus: fils })
        : actualisation.promise;
    if (chemin === "/threads/1%2Fplan")
      return Response.json([entete("1", "Noa", "Plan ancien")]);
    if (chemin === "/threads/2%2Fjardin")
      return Response.json([entete("2", "Jules", "Jardin actuel")]);
    if (chemin === "/messages/1") return tardif.promise;
    if (chemin === "/messages/2") return corps("Lettre présente");
    throw Error(chemin);
  });
  const initial = await vue.findByRole("button", { name: /1\/plan/ });
  const barre = vue.getByRole("button", { name: "Menu" });
  fireEvent.click(initial);
  await waitFor(() =>
    expect(fetch.mock.calls.some(([c]) => c.endsWith("/messages/1"))).toBe(
      true,
    ),
  );
  fireEvent.click(vue.getByRole("button", { name: /Retour/ }));
  const suivante = await vue.findByRole("button", { name: /2\/jardin/ });
  expect(
    fetch.mock.calls.filter(([c]) => c.endsWith("/views/reception")),
  ).toHaveLength(2);
  expect(document.activeElement?.getAttribute("data-fil")).toBe(plan);
  fireEvent.click(suivante);
  await vue.findByRole("heading", { name: "Jardin actuel" });
  const doc = () =>
    (vue.getByTitle("Contenu du message") as HTMLIFrameElement).srcdoc;
  await waitFor(() => expect(doc()).toContain("Lettre présente"));
  tardif.resolve(corps("Lettre dépassée"));
  actualisation.resolve(Response.json(reception));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(doc()).not.toContain("Lettre dépassée");
  expect(vue.getByRole("button", { name: "Menu" })).toBe(barre);
});

it("ouvre une lettre par son lien direct et permet de reprendre une lecture interrompue", async () => {
  let premier = true,
    marquage = true;
  const { vue, fetch } = afficher(`${racine(plan)}/message/1`, (chemin) => {
    if (chemin.startsWith("/threads/"))
      return Response.json([
        {
          ...lea,
          objet: "Ancien",
          id: "1",
          nom: null,
          adresse: "ancien@recette.test",
        },
        entete("2", "Noa", ""),
      ]);
    if (chemin === "/messages/1/open") {
      if (marquage) {
        marquage = false;
        return new Response(null, { status: 503 });
      }
      return Response.json({ fil: plan, revision: 4, non_lus: {} });
    }
    if (chemin === "/messages/1") {
      if (premier) {
        premier = false;
        return Response.json({ etat: "indisponible" });
      }
      return corps("Retrouvé");
    }
    if (chemin === "/views/reception") return Response.json(reception);
    throw Error(chemin);
  });
  await vue.findByRole("heading", { name: "Sans objet" });
  expect(
    await vue.findByRole("button", { name: "ancien@recette.test" }),
  ).toBeTruthy();
  const indispo = await vue.findByText(/Contenu disponible à la reconnexion/);
  const annule = await vue.findByText(/La messagerie n’a pas confirmé/);
  fireEvent.click(within(indispo).getByRole("button"));
  fireEvent.click(within(annule).getByRole("button"));
  await waitFor(() =>
    expect(
      (vue.getByTitle("Contenu du message") as HTMLIFrameElement).srcdoc,
    ).toContain("Retrouvé"),
  );
  expect(vue.queryByText(/La messagerie n’a pas confirmé/)).toBeNull();
  rejoindre(`${racine(lettre)}/message/absent`);
  await vue.findByText(/Ce fil est indisponible/);
  expect(fetch.mock.calls.some(([p]) => p.includes("/messages/absent"))).toBe(
    false,
  );
  const length = history.length;
  fireEvent.click(vue.getByRole("button", { name: /Retour/ }));
  await vue.findByRole("heading", { name: "Réception" });
  expect(location.pathname).toBe("/reception");
  expect(history.length).toBe(length);
});
