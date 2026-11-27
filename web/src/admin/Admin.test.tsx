import { fireEvent, render, waitFor, within } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import App from "../App";
import type { Diagnostic, Profile } from "../sync";

const preset: Profile = {
  name: "Infomaniak",
  imap: { host: "mail.infomaniak.com", port: 993, tls: "implicit" },
  smtp: { host: "mail.infomaniak.com", port: 587, tls: "starttls" },
  domains: [],
  active: true,
  local_test: false,
};
const local: Profile = {
  ...preset,
  name: "Banc local",
  imap: { host: "127.0.0.1", port: 3993, tls: "implicit" },
  smtp: { host: "127.0.0.1", port: 3465, tls: "implicit" },
  local_test: true,
  domains: ["recette.test"],
};
const ok = {
  resolution: "ok",
  connection: "ok",
  tls: "ok",
  authentication: "ok",
} as const;
const failed: Diagnostic = {
  imap: {
    resolution: "failed",
    connection: "skipped",
    tls: "skipped",
    authentication: "skipped",
  },
  smtp: ok,
};
const token = "acces-local-fictif";

function mount() {
  window.history.replaceState(null, "", "/admin");
  const fetchMock = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetchMock);
  const view = render(() => <App />);
  const input = (label: string, value: string) =>
    fireEvent.input(view.getByLabelText(label), { target: { value } });
  const click = (label: string) =>
    fireEvent.click(view.getByRole("button", { name: label }));
  input("Jeton d’administration", token);
  return { fetchMock, view, click, input };
}

afterEach(() => {
  window.history.replaceState(null, "", "/");
});

it("attend le serveur pendant l’ouverture et le diagnostic, puis permet un nouvel essai", async () => {
  const { fetchMock, view, click, input } = mount();
  let grantAccess: (response: Response) => void = () => {
    throw new Error("Demande d’accès absente");
  };
  fetchMock.mockReturnValueOnce(
    new Promise<Response>((resolve) => {
      grantAccess = resolve;
    }),
  );
  click("Ouvrir l’administration");
  const enter = view.getByRole("button", {
    name: "Ouvrir l’administration",
  }) as HTMLButtonElement;
  await waitFor(() => expect(enter.disabled).toBe(true));
  fireEvent.submit(enter.form as HTMLFormElement);
  expect(fetchMock).toHaveBeenCalledTimes(1);
  grantAccess(Response.json({ servers: [], presets: [preset] }));
  await view.findByText("Aucun serveur configuré.");
  click("Ajouter un serveur");
  await view.findByLabelText("Identifiant de messagerie");
  input("Identifiant de messagerie", "essai");
  input("Mot de passe de messagerie", "test");
  const test = view.getByRole("button", {
    name: "Tester",
  }) as HTMLButtonElement;
  let interrupt: (reason: Error) => void = () => {
    throw new Error("Diagnostic absent");
  };
  fetchMock.mockReturnValueOnce(
    new Promise<Response>((_, reject) => {
      interrupt = reject;
    }),
  );
  fireEvent.submit(test.form as HTMLFormElement);
  await view.findByText("Diagnostic en cours…");
  fireEvent.submit(test.form as HTMLFormElement);
  expect(fetchMock).toHaveBeenCalledTimes(2);
  interrupt(new Error("Connexion interrompue."));
  await view.findByText("Connexion interrompue.");
  fetchMock.mockResolvedValueOnce(Response.json({ imap: ok, smtp: ok }));
  click("Tester");
  await view.findByRole("table", { name: "Diagnostic de connexion" });
});

it("conserve l’ordre des serveurs et remplace uniquement celui qui vient d’être enregistré", async () => {
  const { fetchMock, view, click, input } = mount();
  const servers = [
    { ...local, id: 2, name: "Premier" },
    { ...local, id: 4, name: "Second" },
  ];
  fetchMock.mockResolvedValueOnce(
    Response.json({ servers, presets: [preset, local] }),
  );
  click("Ouvrir l’administration");
  await view.findByText("Premier");
  click("Modifier Premier");
  await view.findByLabelText("Identifiant de messagerie");
  input("Nom du serveur", "Modifié");
  input("Identifiant de messagerie", "essai");
  input("Mot de passe de messagerie", "test");
  fetchMock.mockResolvedValueOnce(
    Response.json({
      server: { ...servers[0], name: "Modifié" },
      diagnostic: { imap: ok, smtp: ok },
    }),
  );
  click("Tester et enregistrer");
  await view.findByText("Profil enregistré.");
  expect(
    view
      .getAllByRole("listitem")
      .map((li) => li.querySelector("strong")?.textContent),
  ).toEqual(["Modifié", "Second"]);
});

it("réserve le catalogue au jeton saisi et laisse réessayer après un refus", async () => {
  const { fetchMock, view, click } = mount();
  fetchMock.mockResolvedValueOnce(
    Response.json(
      { code: "unauthorized", message: "Jeton refusé." },
      { status: 401 },
    ),
  );
  click("Ouvrir l’administration");
  await view.findByText("Jeton refusé.");
  expect(view.queryByText("Ajouter un serveur")).toBeNull();
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe("/api/admin/v1/servers");
  expect(init?.method).toBe("GET");
  expect(init?.credentials).toBe("omit");
  expect(new Headers(init?.headers).get("Authorization")).toBe(
    `Bearer ${token}`,
  );
  fetchMock.mockResolvedValueOnce(
    Response.json({ servers: [], presets: [preset] }),
  );
  click("Ouvrir l’administration");
  await view.findByText("Aucun serveur configuré.");
  click("Ajouter un serveur");
  await view.findByRole("button", { name: "Annuler" });
  expect(view.queryByRole("option", { name: "Banc local" })).toBeNull();
  click("Annuler");
  await view.findByText("Aucun serveur configuré.");
  click("Fermer l’administration");
  expect(
    ((await view.findByLabelText("Jeton d’administration")) as HTMLInputElement)
      .value,
  ).toBe("");
});

it("teste, crée et modifie un profil en gardant la configuration après un échec", async () => {
  const { fetchMock, view, click, input } = mount();
  fetchMock.mockResolvedValueOnce(
    Response.json({ servers: [], presets: [preset, local] }),
  );
  click("Ouvrir l’administration");
  await view.findByText("Aucun serveur configuré.");
  click("Ajouter un serveur");
  await view.findByRole("button", { name: "Annuler" });
  const smtp = within(view.getByRole("group", { name: "SMTP" }));
  expect((smtp.getByLabelText("Chiffrement") as HTMLSelectElement).value).toBe(
    "starttls",
  );
  fireEvent.change(view.getByLabelText("Préréglage"), {
    target: { value: "1" },
  });
  await view.findByText("Le certificat du banc local n’est pas vérifié.");
  input("Identifiant de messagerie", "diagnostic");
  input("Mot de passe de messagerie", "essai");
  fetchMock.mockResolvedValueOnce(Response.json(failed));
  click("Tester");
  await view.findByRole("table", { name: "Diagnostic de connexion" });
  const row = view.getByRole("row", { name: /Résolution DNS/ });
  expect(
    within(row)
      .getAllByRole("cell")
      .map((c) => c.textContent),
  ).toEqual(["Échec", "Réussi"]);
  expect(view.getAllByText("Non testé")).toHaveLength(3);
  expect(fetchMock.mock.calls[1][0]).toBe("/api/admin/v1/servers/test");
  expect(JSON.parse(String(fetchMock.mock.calls[1][1]?.body))).toEqual({
    profile: local,
    credentials: { username: "diagnostic", password: "essai" },
  });

  const server = { ...local, id: 7 };
  fetchMock.mockResolvedValueOnce(
    Response.json(
      { server, diagnostic: { imap: ok, smtp: ok } },
      { status: 201 },
    ),
  );
  click("Tester et enregistrer");
  await view.findByText("Profil enregistré.");
  expect(fetchMock.mock.calls[2][0]).toBe("/api/admin/v1/servers");
  expect(fetchMock.mock.calls[2][1]?.method).toBe("POST");
  click("Modifier Banc local");
  await view.findByRole("heading", { name: "Modifier le serveur" });
  expect(
    (view.getByLabelText("Mot de passe de messagerie") as HTMLInputElement)
      .value,
  ).toBe("");
  input("Nom du serveur", "Nouveau");
  input("Domaines associés (séparés par des virgules)", "un.test, deux.test");
  input("Identifiant de messagerie", "essai");
  input("Mot de passe de messagerie", "refus");
  fireEvent.click(view.getByLabelText("Profil actif"));
  fetchMock.mockResolvedValueOnce(
    Response.json({ server: null, diagnostic: failed }, { status: 422 }),
  );
  click("Tester et enregistrer");
  await view.findByText("Enregistrement refusé. Vérifiez les étapes en échec.");
  const [url, init] = fetchMock.mock.calls[3];
  expect(url).toBe("/api/admin/v1/servers/7");
  expect(init?.method).toBe("PUT");
  expect(JSON.parse(String(init?.body)).profile).toMatchObject({
    name: "Nouveau",
    domains: ["un.test", "deux.test"],
    active: false,
  });
  input("Mot de passe de messagerie", "encore");
  await waitFor(() =>
    expect(view.queryByText("Diagnostic de connexion")).toBeNull(),
  );
  click("Annuler");
  await view.findByRole("button", { name: "Modifier Banc local" });
});
