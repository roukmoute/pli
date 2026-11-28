import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { afterAll, afterEach, beforeAll, expect, it, vi } from "vitest";
import catalogue from "../../tests/dossiers.json";
import { fil, page } from "../../tests/fils";
import { reponseRetenue } from "../../tests/reponse-retenue";
import App from "../App";
import { connexionFr } from "../i18n/fr";
import type { Dossier, Fil } from "../sync";
import Courrier from "./Courrier";

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
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
});

const { inbox, atelier, classement, envoyes, archives, corbeille, reclames } =
  catalogue as Record<string, Dossier>;

interface Demande {
  id: string;
  type: string;
  vue: string;
  cibles: string[];
  destination?: number;
}

interface Options {
  fils: Fil[];
  dossiers?: Dossier[];
  route?: string;
  serie?: boolean;
}

// Le faux serveur garde les fils de la boîte et conclut chaque archivage dès sa réponse.
function ouvrir(options: Options) {
  const serveur = {
    fils: options.fils,
    dossiers: options.dossiers || [inbox, archives],
    revision: 2,
    refus: [] as string[],
    retenue: Promise.resolve(),
    demandes: [] as Demande[],
    choix: [] as number[],
    choixRefuse: false,
    // La liste entière peut compter plus de fils que la page servie.
    horsPage: 0,
    total: () => serveur.fils.length + serveur.horsPage,
  };
  const liste = (portee: object) =>
    Response.json({
      ...page(serveur.fils, null, serveur.revision),
      vue: undefined,
      total: serveur.total(),
      ...portee,
    });
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
      const chemin = String(url).replace("/api/v1", "");
      if (chemin === "/session")
        return Response.json({
          account_id: 3,
          address: "noa@recette.test",
          expires_at: Math.floor(Date.now() / 1000) + 300,
        });
      if (chemin === "/folders")
        return Response.json({ folders: serveur.dossiers });
      if (chemin === "/folders/archive") {
        const { dossier }: { dossier: number } = JSON.parse(String(init?.body));
        serveur.choix.push(dossier);
        if (serveur.choixRefuse)
          return Response.json(
            { code: "dossier_invalide", message: "Refusé." },
            { status: 422 },
          );
        serveur.dossiers = serveur.dossiers.map((d) =>
          d.id === dossier ? { ...d, role: "archive" } : d,
        );
        return Response.json({ folders: serveur.dossiers });
      }
      if (chemin === "/views/reception") return liste({ vue: "reception" });
      if (chemin === "/views/flux") return liste({ vue: "flux" });
      const dossier = chemin.match(/^\/folders\/(\d+)\/messages$/);
      if (dossier) return liste({ dossier: Number(dossier[1]) });
      if (chemin === "/operations") {
        const demande: Demande = JSON.parse(String(init?.body));
        serveur.demandes.push(demande);
        await serveur.retenue;
        const confirmes = demande.cibles.filter(
          (id) => !serveur.refus.includes(id),
        );
        // Un vidage confirmé emporte toute la liste, les autres départs leurs seuls fils.
        const vide = demande.type === "vider_dossier" && confirmes.length;
        serveur.fils = serveur.fils.filter(
          (f) => !vide && !confirmes.includes(f.id),
        );
        if (vide) serveur.horsPage = 0;
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
      if (chemin.startsWith("/threads/")) {
        const id = decodeURIComponent(chemin.slice("/threads/".length));
        // Le fil « duo » réunit deux lettres, les autres une seule.
        const lettres = id === "duo" ? ["duo-1", "duo-2"] : [`${id}-1`];
        return Response.json(
          lettres.map((lettre) => ({
            id: lettre,
            objet: `Lettre ${id}`,
            nom: "Noa",
            adresse: "noa@recette.test",
            destinataires: {},
            date: 1735689600,
            non_lu: false,
          })),
        );
      }
      if (chemin.startsWith("/messages/"))
        return Response.json({
          etat: "disponible",
          version: 1,
          contenu: { html: "<p>Texte</p>", images: [], pieces: [] },
        });
      throw new Error(`Route inattendue ${url}`);
    }),
  );
  window.history.replaceState(null, "", options.route || "/reception");
  const view = render(() =>
    options.serie ? (
      <Courrier
        compte={3}
        adresse="noa@recette.test"
        textes={connexionFr}
        fermer={async () => {}}
        echec={false}
        lectureEnSerie
      />
    ) : (
      <App />
    ),
  );
  const ligne = (id: string) =>
    view.getByRole("button", { name: new RegExp(`Lettre ${id}`) });
  const absente = (id: string) =>
    view.queryByRole("button", { name: new RegExp(`Lettre ${id}`) }) === null;
  return {
    view,
    serveur,
    ligne,
    absente,
    avatar: (id: string) =>
      within(
        ligne(id).parentElement as HTMLElement,
      ).getByRole<HTMLInputElement>("checkbox"),
    section: (id: string) => ligne(id).closest("section")?.dataset.section,
    envois: () =>
      serveur.demandes.map(({ type, vue, cibles }) => [type, vue, ...cibles]),
    commande: (nom: string) => view.queryByRole("button", { name: nom }),
    // Le catalogue des dossiers se charge quand la commande apparaît.
    // Une fois reçu, le geste n’attend plus le réseau.
    async archiver(commande = "Archiver la sélection") {
      const bouton = await view.findByRole("button", { name: commande });
      await new Promise((reprendre) => setTimeout(reprendre, 0));
      fireEvent.click(bouton);
    },
    retenir() {
      const retenue = reponseRetenue<void>();
      serveur.retenue = retenue.promise;
      return () => retenue.resolve();
    },
  };
}

const lignes = () =>
  Array.from(document.querySelectorAll<HTMLElement>("[data-fil]")).map(
    (ligne) => ligne.dataset.fil,
  );

it("archive la sélection dès le geste et ne propose à la reprise que le fil refusé", async () => {
  const {
    view,
    serveur,
    ligne,
    absente,
    avatar,
    section,
    envois,
    archiver,
    retenir,
  } = ouvrir({ fils: [fil("a", 40, false), fil("b", 30), fil("c", 20)] });
  await view.findByRole("button", { name: /Lettre a/ });
  serveur.refus = ["c"];
  const liberer = retenir();
  fireEvent.click(avatar("b"));
  fireEvent.click(avatar("c"));
  const commande = await view.findByRole("button", {
    name: "Archiver la sélection",
  });
  const globale = view.getByRole<HTMLInputElement>("checkbox", {
    name: "Sélectionner la page",
  });
  commande.focus();
  await archiver();
  // La réponse est retenue : B et C ont déjà quitté la liste, A garde sa section.
  expect(lignes()).toEqual(["a"]);
  expect(section("a")).toBe("deja_vus");
  expect(within(ligne("a")).getByText("Lu")).toBeTruthy();
  expect(envois()).toEqual([["archiver", "reception", "b", "c"]]);
  // L’état transitoire se lit sans couleur, à côté du décompte ajusté.
  expect(view.getByRole("status").textContent).toBe(
    "0 fils sélectionnés sur 1Archivage en attente pour 2 fils.",
  );
  expect(view.queryByRole("alert")).toBeNull();
  // Les commandes du lot ont disparu : le focus rejoint la case de la page.
  expect(commande.isConnected).toBe(false);
  expect(document.activeElement).toBe(globale);
  liberer();
  expect((await view.findByRole("alert")).textContent).toBe(
    "Modification non confirmée pour 1 fil.",
  );
  await waitFor(() => expect(lignes()).toEqual(["c", "a"]));
  // Seul C, refusé, revient coché. B est parti pour de bon.
  expect(section("c")).toBe("nouveaux");
  expect(avatar("c").checked).toBe(true);
  expect(avatar("a").checked).toBe(false);
  expect(absente("b")).toBe(true);
  expect(view.getByRole("status").textContent).toBe("1 fil sélectionné sur 2");
  // La reprise n’envoie que le fil refusé.
  serveur.refus = [];
  await archiver();
  await waitFor(() => expect(lignes()).toEqual(["a"]));
  expect(envois()[1]).toEqual(["archiver", "reception", "c"]);
  await waitFor(() => expect(view.queryByRole("alert")).toBeNull());
  // L’état transitoire s’efface quand la messagerie a confirmé.
  await waitFor(() =>
    expect(view.getByRole("status").textContent).toBe(
      "0 fils sélectionnés sur 1",
    ),
  );
});

it("donne le focus au titre quand l’archivage vide la liste", async () => {
  const { view, avatar, archiver, retenir } = ouvrir({ fils: [fil("b", 30)] });
  await view.findByRole("button", { name: /Lettre b/ });
  const liberer = retenir();
  fireEvent.click(avatar("b"));
  const commande = await view.findByRole("button", {
    name: "Archiver la sélection",
  });
  commande.focus();
  const titre = view.getByRole("heading", { name: "Réception" });
  const focus = vi.spyOn(titre, "focus");
  await archiver();
  expect(lignes()).toEqual([]);
  expect(document.activeElement).toBe(titre);
  expect(focus.mock.calls).toEqual([[{ preventScroll: true }]]);
  expect(view.getByRole("status").textContent).toBe(
    "Archivage en attente pour 1 fil.",
  );
  expect(view.getByRole("button", { name: "Menu" })).toBeTruthy();
  liberer();
  await waitFor(() => expect(view.getByRole("status").textContent).toBe(""));
  expect(view.getByText("Aucun nouveau fil.")).toBeTruthy();
});

it("ne propose pas d’archiver depuis le dossier qui reçoit les archives", async () => {
  const { view, avatar } = ouvrir({
    fils: [fil("b", 30)],
    route: "/dossier/7",
  });
  await view.findByRole("button", { name: /Lettre b/ });
  fireEvent.click(avatar("b"));
  await view.findByRole("button", { name: "Marquer comme lu" });
  await view.findByRole("heading", { name: "Archives" });
  await waitFor(() =>
    expect(
      view.queryByRole("button", { name: "Archiver la sélection" }),
    ).toBeNull(),
  );
});

it("archive le fil ouvert puis reprend la liste sur la ligne voisine", async () => {
  const { view, ligne, envois, archiver } = ouvrir({
    fils: [fil("a", 40), fil("duo", 30), fil("c", 20), fil("d", 10, false)],
  });
  fireEvent.click(await view.findByRole("button", { name: /Lettre duo/ }));
  await view.findByRole("heading", { name: "Lettre duo" });
  // Le libellé annonce la portée : ce fil réunit deux lettres.
  await archiver("Archiver le fil");
  await view.findByRole("heading", { name: "Réception" });
  expect(window.location.pathname).toBe("/reception");
  expect(lignes()).toEqual(["a", "c", "d"]);
  expect(envois()).toEqual([["archiver", "reception", "duo"]]);
  // La ligne qui suivait le fil archivé reçoit le focus.
  expect(document.activeElement).toBe(ligne("c"));
  expect(view.queryByRole("alert")).toBeNull();

  // Un fil d’une seule lettre s’archive comme un message.
  fireEvent.click(ligne("d"));
  await view.findByRole("heading", { name: "Lettre d" });
  expect(view.queryByRole("button", { name: "Archiver le fil" })).toBeNull();
  await archiver("Archiver le message");
  await view.findByRole("heading", { name: "Réception" });
  await waitFor(() => expect(lignes()).toEqual(["a", "c"]));
  expect(envois()[1]).toEqual(["archiver", "reception", "d"]);
  // « Déjà vus » s’est vidé : la dernière ligne de « Nouveaux » reprend le focus.
  expect(document.activeElement).toBe(ligne("c"));
});

it("ramène en Réception un fil refusé depuis sa lecture et le propose à la reprise", async () => {
  const { view, serveur, avatar, archiver } = ouvrir({
    fils: [fil("a", 40), fil("b", 30)],
  });
  serveur.refus = ["b"];
  fireEvent.click(await view.findByRole("button", { name: /Lettre b/ }));
  await view.findByRole("heading", { name: "Lettre b" });
  await archiver("Archiver le message");
  await view.findByRole("heading", { name: "Réception" });
  expect((await view.findByRole("alert")).textContent).toBe(
    "Modification non confirmée pour 1 fil.",
  );
  await waitFor(() => expect(lignes()).toEqual(["a", "b"]));
  expect(avatar("b").checked).toBe(true);
  expect(avatar("a").checked).toBe(false);
});

it("archive dans la vue d’origine du fil et retombe sur Réception après un lien direct", async () => {
  const flux = ouvrir({ fils: [fil("b", 30)], route: "/flux" });
  fireEvent.click(await flux.view.findByRole("button", { name: /Lettre b/ }));
  await flux.view.findByRole("heading", { name: "Lettre b" });
  await flux.archiver("Archiver le message");
  await flux.view.findByRole("heading", { name: "Flux" });
  expect(flux.envois()).toEqual([["archiver", "flux", "b"]]);
  flux.view.unmount();

  const direct = ouvrir({
    fils: [fil("a", 40), fil("b", 30)],
    route: "/fil/b",
  });
  await direct.view.findByRole("heading", { name: "Lettre b" });
  await direct.archiver("Archiver le message");
  await direct.view.findByRole("heading", { name: "Réception" });
  expect(window.location.pathname).toBe("/reception");
  expect(direct.envois()).toEqual([["archiver", "reception", "b"]]);
  await waitFor(() => expect(lignes()).toEqual(["a"]));
  direct.view.unmount();

  // Ouvert depuis le dossier des archives, le fil n’a plus à être archivé.
  const classe = ouvrir({ fils: [fil("b", 30)], route: "/dossier/7" });
  fireEvent.click(await classe.view.findByRole("button", { name: /Lettre b/ }));
  await classe.view.findByRole("heading", { name: "Lettre b" });
  await classe.view.findAllByRole("button", { name: /Répondre/ });
  expect(
    classe.view.queryByRole("button", { name: "Archiver le message" }),
  ).toBeNull();
});

it("ouvre le fil suivant après l’archivage quand la lecture en série est active", async () => {
  const { view, ligne, envois, archiver } = ouvrir({
    fils: [fil("a", 40), fil("b", 30), fil("c", 20)],
    serie: true,
  });
  fireEvent.click(await view.findByRole("button", { name: /Lettre b/ }));
  await view.findByRole("heading", { name: "Lettre b" });
  await archiver("Archiver le message");
  // Le fil qui suivait dans la liste s’ouvre à la place du fil archivé.
  await view.findByRole("heading", { name: "Lettre c" });
  expect(window.location.pathname).toBe("/fil/c");
  // Il a pris la place du fil archivé : le retour mène à la liste, sur sa ligne.
  fireEvent.click(view.getByRole("button", { name: /Retour/ }));
  await view.findByRole("heading", { name: "Réception" });
  expect(lignes()).toEqual(["a", "c"]);
  expect(document.activeElement).toBe(ligne("c"));

  fireEvent.click(ligne("c"));
  await view.findByRole("heading", { name: "Lettre c" });
  await archiver("Archiver le message");
  await view.findByRole("heading", { name: "Lettre a" });
  expect(window.location.pathname).toBe("/fil/a");
  // Dernier fil de la série : la liste reprend la main, vide.
  await archiver("Archiver le message");
  const titre = await view.findByRole("heading", { name: "Réception" });
  expect(envois()).toEqual([
    ["archiver", "reception", "b"],
    ["archiver", "reception", "c"],
    ["archiver", "reception", "a"],
  ]);
  await waitFor(() => expect(lignes()).toEqual([]));
  expect(document.activeElement).toBe(titre);
});

it("demande le dossier des archives une fois, le retient et archive", async () => {
  const { view, serveur, avatar, envois, archiver } = ouvrir({
    fils: [fil("a", 40), fil("b", 30)],
    dossiers: [inbox, atelier, classement, envoyes],
  });
  await view.findByRole("button", { name: /Lettre a/ });
  fireEvent.click(avatar("b"));
  const commande = await view.findByRole("button", {
    name: "Archiver la sélection",
  });
  commande.focus();
  await archiver();
  const demande = await view.findByRole("dialog", {
    name: "Dossier des archives",
  });
  // Rien n’est parti : la liste garde ses fils tant que la destination manque.
  expect(lignes()).toEqual(["a", "b"]);
  expect(envois()).toEqual([]);
  const proposes = within(demande)
    .getAllByRole("button")
    .map((bouton) => bouton.textContent);
  expect(proposes).toEqual(["Atelier", "Annuler"]);
  expect(within(demande).queryByRole("alert")).toBeNull();

  // Renoncer ferme la demande et rend le focus à la commande.
  const focus = vi.spyOn(commande, "focus");
  fireEvent.click(within(demande).getByRole("button", { name: "Annuler" }));
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(document.activeElement).toBe(commande);
  expect(focus.mock.calls).toEqual([[{ preventScroll: true }]]);
  expect(serveur.choix).toEqual([]);
  expect(lignes()).toEqual(["a", "b"]);

  // Un refus du serveur laisse la demande ouverte et l’annonce.
  serveur.choixRefuse = true;
  await archiver();
  const reprise = await view.findByRole("dialog");
  fireEvent.click(within(reprise).getByRole("button", { name: "Atelier" }));
  expect((await within(reprise).findByRole("alert")).textContent).toBe(
    "Ce dossier n’a pas pu être retenu.",
  );
  expect(envois()).toEqual([]);
  serveur.choixRefuse = false;
  fireEvent.click(within(reprise).getByRole("button", { name: "Atelier" }));
  await waitFor(() => expect(lignes()).toEqual(["a"]));
  expect(view.queryByRole("dialog")).toBeNull();
  expect(serveur.choix).toEqual([atelier.id, atelier.id]);
  expect(envois()).toEqual([["archiver", "reception", "b"]]);

  // Le choix est retenu : l’archivage suivant part sans rien demander.
  fireEvent.click(avatar("a"));
  await archiver();
  expect(lignes()).toEqual([]);
  expect(view.queryByRole("dialog")).toBeNull();
  expect(serveur.choix).toHaveLength(2);
});

it("explique qu’aucun dossier ne peut recevoir les archives", async () => {
  const { view, avatar, archiver } = ouvrir({
    fils: [fil("b", 30)],
    dossiers: [inbox, classement, envoyes],
  });
  await view.findByRole("button", { name: /Lettre b/ });
  fireEvent.click(avatar("b"));
  await view.findByRole("button", { name: "Archiver la sélection" });
  await archiver();
  const demande = await view.findByRole("dialog");
  expect(within(demande).getAllByRole("button")).toHaveLength(1);
  expect(demande.textContent).toContain(
    "Aucun dossier personnel ne peut recevoir les archives.",
  );
  // La touche Échap renonce comme le bouton.
  fireEvent(demande, new Event("cancel", { cancelable: true }));
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(lignes()).toEqual(["b"]);
});

const tous = [
  inbox,
  atelier,
  classement,
  envoyes,
  archives,
  corbeille,
  reclames,
];

it("supprime la sélection sans confirmation et ne ramène que le fil refusé", async () => {
  const {
    view,
    serveur,
    avatar,
    envois,
    archiver: lancer,
    retenir,
    commande,
  } = ouvrir({
    fils: [fil("a", 40, false), fil("b", 30), fil("c", 20)],
    dossiers: tous,
  });
  await view.findByRole("button", { name: /Lettre a/ });
  serveur.refus = ["c"];
  const liberer = retenir();
  fireEvent.click(avatar("b"));
  fireEvent.click(avatar("c"));
  await lancer("Supprimer la sélection");
  // Aucune confirmation : les fils partent pour la Corbeille dès le geste.
  expect(view.queryByRole("dialog")).toBeNull();
  expect(lignes()).toEqual(["a"]);
  expect(envois()).toEqual([["corbeille", "reception", "b", "c"]]);
  expect(view.getByRole("status").textContent).toBe(
    "0 fils sélectionnés sur 1Suppression en attente pour 2 fils.",
  );
  liberer();
  expect((await view.findByRole("alert")).textContent).toBe(
    "Modification non confirmée pour 1 fil.",
  );
  await waitFor(() => expect(lignes()).toEqual(["c", "a"]));
  expect(avatar("c").checked).toBe(true);
  // Hors de la Corbeille, rien ne se restaure ni ne s’efface pour de bon.
  for (const nom of [
    "Restaurer la sélection",
    "Supprimer définitivement",
    "Vider le dossier",
  ])
    expect(commande(nom)).toBeNull();
});

it("restaure depuis la Corbeille et demande un dossier quand l’origine manque", async () => {
  const {
    view,
    serveur,
    avatar,
    envois,
    archiver: lancer,
    retenir,
    commande,
  } = ouvrir({
    fils: [fil("a", 40), { ...fil("b", 30), sans_origine: true }, fil("c", 20)],
    dossiers: tous,
    route: "/dossier/8",
  });
  await view.findByRole("heading", { name: "Papierkorb" });
  await view.findByRole("button", { name: /Lettre a/ });
  const liberer = retenir();
  fireEvent.click(avatar("a"));
  await lancer("Restaurer la sélection");
  // Le fil sait où revenir : il part sans rien demander.
  expect(view.queryByRole("dialog")).toBeNull();
  expect(lignes()).toEqual(["b", "c"]);
  expect(envois()).toEqual([["restaurer", "dossier/8", "a"]]);
  expect(serveur.demandes[0].destination).toBeUndefined();
  expect(view.getByRole("status").textContent).toBe(
    "0 fils sélectionnés sur 2Restauration en attente pour 1 fil.",
  );
  liberer();
  await waitFor(() =>
    expect(view.getByRole("status").textContent).toBe(
      "0 fils sélectionnés sur 2",
    ),
  );
  // Dans la Corbeille, on restaure ou on efface : ni archivage ni seconde suppression.
  fireEvent.click(avatar("b"));
  fireEvent.click(avatar("c"));
  await view.findByRole("button", { name: "Restaurer la sélection" });
  expect(commande("Archiver la sélection")).toBeNull();
  expect(commande("Supprimer la sélection")).toBeNull();

  // Le fil b ne sait pas où revenir : rien ne part avant le choix d’un dossier.
  await lancer("Restaurer la sélection");
  const demande = await view.findByRole("dialog", {
    name: "Dossier de restauration",
  });
  expect(lignes()).toEqual(["b", "c"]);
  expect(envois()).toHaveLength(1);
  // Choisir un dossier ne détruit rien : le focus s’ouvre sur le premier choix.
  const renoncer = within(demande).getByRole("button", { name: "Annuler" });
  expect(renoncer.hasAttribute("autofocus")).toBe(false);
  const proposes = within(demande)
    .getAllByRole("button")
    .map((bouton) => bouton.textContent);
  expect(proposes).toEqual([
    "Réception",
    "Atelier",
    "Envoyés",
    "Archives",
    "Annuler",
  ]);
  fireEvent.click(within(demande).getByRole("button", { name: "Annuler" }));
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(envois()).toHaveLength(1);

  await lancer("Restaurer la sélection");
  const reprise = await view.findByRole("dialog");
  fireEvent.click(within(reprise).getByRole("button", { name: "Atelier" }));
  await waitFor(() => expect(lignes()).toEqual([]));
  expect(view.queryByRole("dialog")).toBeNull();
  expect(serveur.demandes[1]).toMatchObject({
    type: "restaurer",
    vue: "dossier/8",
    cibles: ["b", "c"],
    destination: atelier.id,
  });
});

it("fait confirmer la suppression définitive en citant le dossier et le nombre de fils", async () => {
  const {
    view,
    avatar,
    envois,
    archiver: lancer,
    retenir,
  } = ouvrir({
    fils: [fil("a", 40), fil("b", 30), fil("c", 20)],
    dossiers: tous,
    route: "/dossier/8",
  });
  await view.findByRole("heading", { name: "Papierkorb" });
  await view.findByRole("button", { name: /Lettre a/ });
  fireEvent.click(avatar("a"));
  fireEvent.click(avatar("b"));
  const declencheur = await view.findByRole("button", {
    name: "Supprimer définitivement",
  });
  declencheur.focus();
  await lancer("Supprimer définitivement");
  const confirmation = await view.findByRole("dialog", {
    name: "Supprimer définitivement",
  });
  expect(confirmation.textContent).toContain(
    "2 fils seront supprimés définitivement de Papierkorb.",
  );
  // Avant une action définitive, le focus s’ouvre sur « Annuler ».
  const annuler = within(confirmation).getByRole("button", { name: "Annuler" });
  expect(annuler.hasAttribute("autofocus")).toBe(true);
  expect(lignes()).toEqual(["a", "b", "c"]);
  // Annuler ne supprime rien et rend le focus à la commande.
  fireEvent.click(
    within(confirmation).getByRole("button", { name: "Annuler" }),
  );
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(document.activeElement).toBe(declencheur);
  expect(envois()).toEqual([]);
  expect(lignes()).toEqual(["a", "b", "c"]);

  fireEvent.click(avatar("b"));
  await lancer("Supprimer définitivement");
  const seule = await view.findByRole("dialog");
  expect(seule.textContent).toContain(
    "1 fil sera supprimé définitivement de Papierkorb.",
  );
  const liberer = retenir();
  fireEvent.click(
    within(seule).getByRole("button", { name: "Supprimer définitivement" }),
  );
  await waitFor(() => expect(lignes()).toEqual(["b", "c"]));
  expect(view.queryByRole("dialog")).toBeNull();
  expect(envois()).toEqual([["supprimer_definitivement", "dossier/8", "a"]]);
  expect(view.getByRole("status").textContent).toBe(
    "0 fils sélectionnés sur 2Suppression définitive en attente pour 1 fil.",
  );
  liberer();
  await waitFor(() =>
    expect(view.getByRole("status").textContent).toBe(
      "0 fils sélectionnés sur 2",
    ),
  );
});

it("vide la Corbeille après avoir cité son nom et le nombre de fils", async () => {
  const {
    view,
    serveur,
    envois,
    archiver: lancer,
    retenir,
    commande,
  } = ouvrir({
    fils: [fil("a", 40), fil("b", 30)],
    dossiers: tous,
    route: "/dossier/8",
  });
  // Dix autres fils attendent au-delà de la page affichée.
  serveur.horsPage = 10;
  const titre = await view.findByRole("heading", { name: "Papierkorb" });
  const confirmer = async () => {
    await lancer("Vider le dossier");
    const confirmation = await view.findByRole("dialog", {
      name: "Vider le dossier",
    });
    expect(confirmation.textContent).toContain(
      "12 fils seront supprimés définitivement de Papierkorb.",
    );
    return confirmation;
  };
  // La touche Échap renonce : rien n’est supprimé.
  fireEvent(await confirmer(), new Event("cancel", { cancelable: true }));
  await waitFor(() => expect(view.queryByRole("dialog")).toBeNull());
  expect(envois()).toEqual([]);
  expect(lignes()).toEqual(["a", "b"]);

  // La messagerie refuse : la liste revient et le refus s’annonce.
  serveur.refus = ["dossier/8"];
  const refusee = await confirmer();
  fireEvent.click(
    within(refusee).getByRole("button", { name: "Vider le dossier" }),
  );
  expect((await view.findByRole("alert")).textContent).toBe(
    "Le dossier n’a pas été entièrement vidé.",
  );
  await waitFor(() => expect(lignes()).toEqual(["a", "b"]));
  expect(envois()).toEqual([["vider_dossier", "dossier/8", "dossier/8"]]);

  serveur.refus = [];
  const liberer = retenir();
  const acceptee = await confirmer();
  fireEvent.click(
    within(acceptee).getByRole("button", { name: "Vider le dossier" }),
  );
  await waitFor(() => expect(lignes()).toEqual([]));
  expect(view.queryByRole("dialog")).toBeNull();
  expect(view.queryByRole("alert")).toBeNull();
  expect(view.getByRole("status").textContent).toBe(
    "Vidage du dossier en attente.",
  );
  // La liste est vide : le focus rejoint son titre et la commande s’efface.
  expect(document.activeElement).toBe(titre);
  expect(commande("Vider le dossier")).toBeNull();
  liberer();
  await waitFor(() => expect(view.getByRole("status").textContent).toBe(""));
  expect(commande("Vider le dossier")).toBeNull();
  expect(envois()).toHaveLength(2);
});

it("propose de vider les Indésirables mais pas un dossier ordinaire", async () => {
  const reclame = ouvrir({
    fils: [fil("a", 40)],
    dossiers: tous,
    route: "/dossier/10",
  });
  await reclame.view.findByRole("heading", { name: "Spam" });
  await reclame.view.findByRole("button", { name: "Vider le dossier" });
  reclame.view.unmount();
  const classes = ouvrir({
    fils: [fil("a", 40)],
    dossiers: tous,
    route: "/dossier/7",
  });
  await classes.view.findByRole("heading", { name: "Archives" });
  await classes.view.findByRole("button", { name: /Lettre a/ });
  expect(classes.commande("Vider le dossier")).toBeNull();
});

it("supprime ou restaure le fil ouvert selon le dossier d’où il vient", async () => {
  // Depuis Réception, le fil part pour la Corbeille sans confirmation.
  const recus = ouvrir({ fils: [fil("a", 40), fil("b", 30)], dossiers: tous });
  fireEvent.click(await recus.view.findByRole("button", { name: /Lettre b/ }));
  await recus.view.findByRole("heading", { name: "Lettre b" });
  await recus.archiver("Supprimer le message");
  await recus.view.findByRole("heading", { name: "Réception" });
  expect(recus.envois()).toEqual([["corbeille", "reception", "b"]]);
  expect(lignes()).toEqual(["a"]);
  recus.view.unmount();

  // Depuis la Corbeille, il se restaure, ou s’efface après confirmation.
  const jetes = ouvrir({
    fils: [fil("duo", 40), fil("b", 30)],
    dossiers: tous,
    route: "/dossier/8",
  });
  fireEvent.click(
    await jetes.view.findByRole("button", { name: /Lettre duo/ }),
  );
  await jetes.view.findByRole("heading", { name: "Lettre duo" });
  await jetes.view.findByRole("button", { name: "Restaurer le fil" });
  expect(jetes.commande("Archiver le fil")).toBeNull();
  expect(jetes.commande("Supprimer le fil")).toBeNull();
  await jetes.archiver("Restaurer le fil");
  await jetes.view.findByRole("heading", { name: "Papierkorb" });
  expect(jetes.envois()).toEqual([["restaurer", "dossier/8", "duo"]]);

  fireEvent.click(jetes.ligne("b"));
  await jetes.view.findByRole("heading", { name: "Lettre b" });
  await jetes.view.findByRole("button", { name: "Restaurer le message" });
  await jetes.archiver("Supprimer définitivement");
  const confirmation = await jetes.view.findByRole("dialog");
  expect(confirmation.textContent).toContain(
    "1 fil sera supprimé définitivement de Papierkorb.",
  );
  fireEvent.click(
    within(confirmation).getByRole("button", {
      name: "Supprimer définitivement",
    }),
  );
  await jetes.view.findByRole("heading", { name: "Papierkorb" });
  expect(jetes.envois()[1]).toEqual([
    "supprimer_definitivement",
    "dossier/8",
    "b",
  ]);
  await waitFor(() => expect(lignes()).toEqual([]));
});
