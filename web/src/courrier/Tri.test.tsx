import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { expect, it, vi } from "vitest";
import donnees from "../../tests/expediteur-tri.json";
import { creerTri } from "../sync/tri";
import type { Expediteur, ListeTri } from "../sync/tri-api";
import EcranTri from "./Tri";

const atelier = donnees as Expediteur;
const catalogue = (expediteurs: Expediteur[], suite: string | null = null) =>
  Response.json({ compte: 3, revision: 1, expediteurs, suite });
function ouvrir(mode: ListeTri, requete: ReturnType<typeof vi.fn>) {
  vi.stubGlobal("fetch", requete);
  const tri = creerTri(3, vi.fn());
  return render(() => <EcranTri mode={mode} tri={tri} />);
}
const acceptee = {
  adresse: atelier.adresse,
  etat: "accepte",
  destination: "flux",
  revision: 2,
  date_decision: 200,
};

it("identifie l’expéditeur, conserve l’entrée après un échec et affiche le choix concurrent révisable", async () => {
  const source = vi
    .fn()
    .mockResolvedValueOnce(catalogue([atelier]))
    .mockRejectedValueOnce(new Error("Hors ligne"))
    .mockResolvedValueOnce(
      Response.json(
        { compte: 3, revision_compte: 7, decision: acceptee },
        { status: 409 },
      ),
    )
    .mockResolvedValueOnce(
      Response.json({
        compte: 3,
        revision_compte: 8,
        decision: { ...acceptee, etat: "refuse", revision: 3 },
      }),
    );
  const vue = ouvrir("attente", source);
  const ligne = within(
    await vue.findByRole("article", { name: atelier.adresse }),
  );
  expect(ligne.getByRole("heading", { name: "L’Atelier" })).toBeInstanceOf(
    HTMLElement,
  );
  expect(ligne.getByText(atelier.adresse)).toBeInstanceOf(HTMLElement);
  expect(ligne.getByText("Nouvelles du verger")).toBeInstanceOf(HTMLElement);
  expect(ligne.getByText("Lettre de mai")).toBeInstanceOf(HTMLElement);
  expect(
    (ligne.getByLabelText("Accepter vers") as HTMLSelectElement).value,
  ).toBe("reception");
  fireEvent.change(ligne.getByRole("combobox"), {
    target: { value: "justificatifs" },
  });
  fireEvent.click(ligne.getByRole("button", { name: "Accepter" }));
  expect((await vue.findByRole("alert")).textContent).toContain(
    atelier.adresse,
  );
  expect(JSON.parse(source.mock.calls[1][1].body)).toMatchObject({
    destination: "justificatifs",
  });
  fireEvent.click(await vue.findByRole("button", { name: "Refuser" }));
  await vue.findByText(/Une autre décision a été enregistrée/);
  const avis = within(vue.getByRole("alert"));
  expect(avis.getByText(/Une autre décision/).textContent).toContain("Flux");
  expect(vue.queryByRole("article")).toBeNull();
  expect((avis.getByRole("combobox") as HTMLSelectElement).value).toBe("flux");
  fireEvent.click(avis.getByRole("button", { name: "Refuser" }));
  await waitFor(() => expect(vue.queryByRole("alert")).toBeNull());
  expect(JSON.parse(source.mock.lastCall?.[1].body)).toMatchObject({
    decision: "refuse",
    revision_attendue: 2,
  });
  expect(vue.getByText("Aucun nouvel expéditeur à trier.")).toBeInstanceOf(
    HTMLElement,
  );
});

it("reprend le chargement de l’historique et permet de parcourir les lettres puis les adresses", async () => {
  const sansNom = { ...atelier.messages[0], nom: null, objet: "" };
  const entree = {
    ...atelier,
    etat: "refuse" as const,
    messages: [sansNom],
    suivant: { uid: 13, generation: 7 },
  };
  const autre = {
    ...atelier,
    ...acceptee,
    adresse: "verger@recette.test",
    messages: [],
  } as Expediteur;
  const source = vi
    .fn()
    .mockRejectedValueOnce(new Error("Hors ligne"))
    .mockResolvedValueOnce(catalogue([entree], entree.adresse))
    .mockResolvedValueOnce(
      Response.json({ messages: [atelier.messages[1]], suivant: null }),
    )
    .mockResolvedValueOnce(catalogue([autre]));
  const vue = ouvrir("historique", source);
  fireEvent.click(within(await vue.findByRole("alert")).getByRole("button"));
  const article = await vue.findByRole("article", { name: entree.adresse });
  const ligne = within(article);
  expect(ligne.getByRole("heading", { name: entree.adresse })).toBeInstanceOf(
    HTMLElement,
  );
  expect(ligne.getByText("Refusé")).toBeInstanceOf(HTMLElement);
  fireEvent.click(ligne.getByText("Messages de cet expéditeur"));
  const details = article.querySelector("details");
  if (!details) throw new Error("Historique absent");
  const lettres = within(details);
  fireEvent.click(
    lettres.getByRole("button", { name: "Afficher les messages suivants" }),
  );
  await waitFor(() =>
    expect(source.mock.calls.map(([url]) => url)).toContain(
      "/api/v1/screener/messages?adresse=atelier%40recette.test&avant=13&generation=7",
    ),
  );
  await vue.findByText("Lettre de mars");
  expect(details.isConnected).toBe(true);
  expect(await lettres.findByText("Lettre de mars")).toBeInstanceOf(
    HTMLElement,
  );
  expect(lettres.queryByRole("button")).toBeNull();
  fireEvent.click(
    vue.getByRole("button", { name: "Afficher les expéditeurs suivants" }),
  );
  const seconde = within(
    await vue.findByRole("article", { name: autre.adresse }),
  );
  expect(seconde.getByText("Flux", { selector: "p" })).toBeInstanceOf(
    HTMLElement,
  );
  fireEvent.click(seconde.getByText("Messages de cet expéditeur"));
  expect(seconde.getByText("Aucun message encore présent.")).toBeInstanceOf(
    HTMLElement,
  );
  expect(vue.queryByText("Afficher les expéditeurs suivants")).toBeNull();
});

it("présente l’historique vide et permet de fermer un avis de concurrence", async () => {
  const source = vi.fn().mockResolvedValueOnce(catalogue([]));
  const vide = ouvrir("historique", source);
  expect(await vide.findByText("Aucune décision enregistrée.")).toBeInstanceOf(
    HTMLElement,
  );
  vide.unmount();
  source.mockResolvedValueOnce(catalogue([atelier])).mockResolvedValueOnce(
    Response.json(
      {
        compte: 3,
        revision_compte: 5,
        decision: { ...acceptee, etat: "refuse" },
      },
      { status: 409 },
    ),
  );
  const vue = ouvrir("attente", source);
  fireEvent.click(await vue.findByRole("button", { name: "Accepter" }));
  const avis = within(await vue.findByRole("alert"));
  expect(avis.getByText(/Une autre décision/).textContent).toContain("Refusé");
  fireEvent.click(avis.getByRole("button", { name: "Fermer l’avis" }));
  await waitFor(() => expect(vue.queryByRole("alert")).toBeNull());
});

it("avance au titre suivant puis au titre de page lors du tri au clavier", async () => {
  const autre = { ...atelier, adresse: "bois@recette.test" };
  const accepteePour = (adresse: string) =>
    Response.json({
      compte: 3,
      revision_compte: 9,
      decision: { ...acceptee, adresse },
    });
  const source = vi
    .fn()
    .mockResolvedValueOnce(catalogue([atelier, autre]))
    .mockResolvedValueOnce(accepteePour(atelier.adresse))
    .mockResolvedValueOnce(accepteePour(autre.adresse));
  const vue = ouvrir("attente", source);
  const titre = vue.getByRole("heading", { name: "Nouveaux expéditeurs" });
  const premier = await vue.findByRole("article", { name: atelier.adresse });
  const second = within(vue.getByRole("article", { name: autre.adresse }));
  const suivant = second.getByRole("heading");
  const focus = vi.spyOn(suivant, "focus");
  fireEvent.click(within(premier).getByRole("button", { name: "Accepter" }));
  expect(document.activeElement).toBe(suivant);
  expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  fireEvent.click(second.getByRole("button", { name: "Refuser" }));
  expect(document.activeElement).toBe(titre);
  await vue.findByText("Aucun nouvel expéditeur à trier.");
});

it("garde la page en cache en cas de coupure et reprend la pagination qui a échoué", async () => {
  const adresse = atelier.adresse;
  const source = vi
    .fn()
    .mockResolvedValueOnce(catalogue([atelier], adresse))
    .mockRejectedValueOnce(new Error("Actualisation interrompue"))
    .mockResolvedValueOnce(catalogue([atelier], adresse))
    .mockRejectedValueOnce(new Error("Pagination interrompue"))
    .mockResolvedValueOnce(
      catalogue([{ ...atelier, adresse: "verger@recette.test" }]),
    );
  vi.stubGlobal("fetch", source);
  const tri = creerTri(3, vi.fn());
  await tri.historique.charger();
  const vue = render(() => <EcranTri tri={tri} mode="historique" />);
  expect(
    vue.getByRole("heading", { name: "Historique du tri" }),
  ).toBeInstanceOf(HTMLElement);
  await vue.findByRole("alert");
  expect(vue.getByRole("article", { name: adresse })).toBeInstanceOf(
    HTMLElement,
  );
  fireEvent.click(within(vue.getByRole("alert")).getByRole("button"));
  await waitFor(() => expect(vue.queryByRole("alert")).toBeNull());
  fireEvent.click(
    vue.getByRole("button", { name: "Afficher les expéditeurs suivants" }),
  );
  const avis = await vue.findByRole("alert");
  fireEvent.click(within(avis).getByRole("button"));
  await vue.findByRole("article", { name: "verger@recette.test" });
  expect(source.mock.calls.slice(-2).map(([url]) => url)).toEqual(
    Array(2).fill(
      "/api/v1/screener/historique?details=true&apres=atelier%40recette.test",
    ),
  );
});

it("annonce le chargement initial et accepte le choix révisé depuis l’avis", async () => {
  let finir!: (r: Response) => void;
  const source = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<Response>((resolve) => {
          finir = resolve;
        }),
    )
    .mockResolvedValueOnce(
      Response.json(
        {
          compte: 3,
          revision_compte: 5,
          decision: { ...acceptee, etat: "refuse" },
        },
        { status: 409 },
      ),
    )
    .mockResolvedValueOnce(
      Response.json({
        compte: 3,
        revision_compte: 6,
        decision: { ...acceptee, revision: 3 },
      }),
    );
  const vue = ouvrir("attente", source);
  expect(vue.getByRole("status").textContent).toBe(
    "Chargement des expéditeurs…",
  );
  await waitFor(() => expect(source).toHaveBeenCalledTimes(1));
  finir(catalogue([atelier]));
  fireEvent.click(await vue.findByRole("button", { name: "Refuser" }));
  const avis = within(await vue.findByRole("alert"));
  fireEvent.click(avis.getByRole("button", { name: "Accepter" }));
  await waitFor(() => expect(vue.queryByRole("alert")).toBeNull());
  expect(JSON.parse(source.mock.lastCall?.[1].body)).toMatchObject({
    decision: "accepte",
    revision_attendue: 2,
  });
});
