import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import receptionVide from "../tests/reception-vide.json";
import App from "./App";
import { getJson } from "./sync/api";

const address = "alice@recette.test";
const password = "secret-fictif";
const session = (expires = Math.floor(Date.now() / 1000) + 300) => ({
  address,
  account_id: 3,
  expires_at: expires,
});

function denied(code = "session_expiree", status = 401) {
  return Response.json({ code, message: "Échec" }, { status });
}

function mount(initial: Response | Promise<Response>) {
  const fetchMock = vi
    .fn<typeof fetch>()
    .mockReturnValueOnce(Promise.resolve(initial));
  vi.stubGlobal("fetch", (url: RequestInfo | URL, init?: RequestInit) => {
    if (url === "/api/v1/views/reception")
      return Promise.resolve(Response.json(receptionVide));
    return fetchMock(url, init);
  });
  const view = render(() => <App />);
  return { view, fetchMock };
}

afterEach(() => {
  document.documentElement.lang = "fr";
  vi.useRealTimers();
});

it("garde l’adresse, replace le focus et envoie une seule demande par soumission", async () => {
  const { view, fetchMock } = mount(denied());
  const submit = (await view.findByRole("button", {
    name: "Se connecter",
  })) as HTMLButtonElement;
  expect(view.queryByText(/Votre session a expiré/)).toBeNull();
  const email = view.getByLabelText(
    "Adresse de messagerie",
  ) as HTMLInputElement;
  const secret = view.getByLabelText(
    "Secret de messagerie",
  ) as HTMLInputElement;
  expect(email.autocomplete).toBe("username");
  expect(email.getAttribute("spellcheck")).toBe("false");
  expect(secret.autocomplete).toBe("current-password");
  fireEvent.input(email, { target: { value: address } });
  fireEvent.input(secret, { target: { value: password } });
  let respond!: (response: Response) => void;
  fetchMock.mockReturnValueOnce(
    new Promise((resolve) => {
      respond = resolve;
    }),
  );
  const form = submit.form as HTMLFormElement;
  fireEvent.submit(form);
  fireEvent.submit(form);
  expect(fetchMock).toHaveBeenCalledTimes(2);
  expect(fetchMock).toHaveBeenLastCalledWith(
    "/api/v1/session",
    expect.objectContaining({
      method: "POST",
      credentials: "same-origin",
      redirect: "error",
      body: JSON.stringify({ address, password }),
    }),
  );
  respond(denied("identifiants_invalides"));
  expect((await view.findByRole("alert")).textContent).toBe(
    "Adresse ou secret de messagerie incorrect.",
  );
  expect(document.activeElement).toBe(secret);
  expect(email.value).toBe(address);
  expect(secret.value).toBe("");
  fireEvent.input(secret, { target: { value: password } });
  fetchMock.mockResolvedValueOnce(denied("service_indisponible", 503));
  fireEvent.submit(form);
  await view.findByText(/service de messagerie est indisponible/);
  expect(document.activeElement).toBe(submit);
  expect(email.value).toBe(address);
  fetchMock.mockResolvedValueOnce(Response.json(session()));
  fireEvent.submit(form);
  await view.findByRole("heading", { name: "Réception" });
  expect(view.queryByRole("alert")).toBeNull();
  view.getByText(address);
  fireEvent.click(view.getByLabelText(address, { selector: "summary" }));
  fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));
  fireEvent.click(view.getByRole("button", { name: "Se déconnecter" }));
  await view.findByRole("heading", { name: "Connexion" });
  expect(view.queryByText(/Votre session a expiré/)).toBeNull();
  expect(fetchMock).toHaveBeenLastCalledWith(
    "/api/v1/session",
    expect.objectContaining({ method: "DELETE" }),
  );
});

it("change la langue avant la connexion et traduit les erreurs", async () => {
  const { view, fetchMock } = mount(denied());
  await view.findByLabelText("Adresse de messagerie");
  fireEvent.change(view.getByLabelText("Langue"), { target: { value: "en" } });
  const email = await view.findByLabelText("Email address");
  expect(document.documentElement.lang).toBe("en");
  fireEvent.input(email, { target: { value: address } });
  fireEvent.input(view.getByLabelText("Email password"), {
    target: { value: password },
  });
  fetchMock.mockResolvedValueOnce(denied("identifiants_invalides"));
  fireEvent.click(view.getByRole("button", { name: "Sign in" }));
  expect((await view.findByRole("alert")).textContent).toBe(
    "Incorrect email address or password.",
  );
  fireEvent.change(view.getByLabelText("Language"), {
    target: { value: "fr" },
  });
  await view.findByText("Adresse ou secret de messagerie incorrect.");
  expect(document.documentElement.lang).toBe("fr");
});

it("restaure une session puis quitte la Réception au premier refus d’une route de données", async () => {
  const { view, fetchMock } = mount(Response.json(session()));
  await view.findByText(address);
  fetchMock.mockResolvedValueOnce(denied());
  await expect(getJson("/courrier", () => null)).rejects.toMatchObject({
    status: 401,
  });
  await view.findByRole("heading", { name: "Connexion" });
  view.getByText("Votre session a expiré. Reconnectez-vous.");
  expect(view.queryByText(address)).toBeNull();
});

it("revérifie la session lorsque l’on revient dans un onglet après une déconnexion", async () => {
  const { view, fetchMock } = mount(Response.json(session()));
  await view.findByText(address);
  fetchMock.mockResolvedValueOnce(denied());
  window.dispatchEvent(new Event("focus"));
  await view.findByRole("heading", { name: "Connexion" });
  expect(view.queryByText(address)).toBeNull();
});

it("garde la vue du même compte et écarte une ancienne vérification de session en échec", async () => {
  const { view, fetchMock } = mount(Response.json(session()));
  const boite = await view.findByRole("heading", { name: "Réception" });
  let rejeter!: (raison: Error) => void;
  fetchMock.mockImplementationOnce(
    () =>
      new Promise((_, reject) => {
        rejeter = reject;
      }),
  );
  window.dispatchEvent(new Event("focus"));
  const adresse = "autre-alias@recette.test";
  fetchMock.mockResolvedValueOnce(
    Response.json({ ...session(), address: adresse }),
  );
  window.dispatchEvent(new Event("pageshow"));
  await view.findByText(adresse);
  rejeter(new TypeError("Ancienne connexion interrompue"));
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  expect(view.getByRole("heading", { name: "Réception" })).toBe(boite);
  view.getByText(adresse);
  expect(view.queryByRole("alert")).toBeNull();
});

it("verrouille une réception ouverte à l’échéance annoncée par le serveur", async () => {
  vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
  vi.setSystemTime(new Date("2026-01-01T12:00:00Z"));
  const { view } = mount(Response.json(session(Date.now() / 1000 + 3)));
  await vi.advanceTimersByTimeAsync(10);
  expect(view.getByRole("heading", { name: "Réception" })).toBeTruthy();
  await vi.advanceTimersByTimeAsync(2989);
  view.getByText(address);
  await vi.advanceTimersByTimeAsync(1);
  view.getByRole("heading", { name: "Connexion" });
  view.getByText("Votre session a expiré. Reconnectez-vous.");
});

it("permet de réessayer après une lecture ou une fermeture indisponible", async () => {
  let finish!: (response: Response) => void;
  const { view, fetchMock } = mount(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  await view.findByText("Ouverture de Pli…");
  expect(view.queryByRole("button", { name: "Réessayer" })).toBeNull();
  finish(denied("service_indisponible", 503));
  const retry = await view.findByRole("button", { name: "Réessayer" });
  fetchMock.mockResolvedValueOnce(Response.json(session()));
  fireEvent.click(retry);
  fireEvent.click(await view.findByLabelText(address, { selector: "summary" }));
  const close = (await view.findByRole("button", {
    name: "Se déconnecter",
  })) as HTMLButtonElement;
  let fail!: (reason: Error) => void;
  fetchMock.mockReturnValueOnce(
    new Promise((_, reject) => {
      fail = reject;
    }),
  );
  fireEvent.click(close);
  fireEvent.click(close);
  expect(fetchMock).toHaveBeenCalledTimes(3);
  fail(new TypeError("Connexion interrompue"));
  await waitFor(() => expect(close.disabled).toBe(false));
  view.getByRole("alert");
  view.getByText(address);
});

it("restaure encore la session une milliseconde avant son échéance", async () => {
  vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
  vi.setSystemTime(new Date("2026-01-01T12:00:00.999Z"));
  const { view } = mount(Response.json(session((Date.now() + 1) / 1000)));
  await vi.advanceTimersByTimeAsync(0);
  view.getByRole("heading", { name: "Réception" });
  await vi.advanceTimersByTimeAsync(1);
  view.getByRole("heading", { name: "Connexion" });
});
