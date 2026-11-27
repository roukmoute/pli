import { expect, it, vi } from "vitest";
import { getJson, onSessionExpired, request } from "./api";

it("appelle l’API v1 et renvoie le résultat du décodeur", async () => {
  const body = { status: "ok" };
  const fetchMock = vi
    .fn<typeof fetch>()
    .mockResolvedValue(Response.json(body));
  vi.stubGlobal("fetch", fetchMock);
  const decode = vi.fn((_body: unknown) => "disponible");

  const result = await getJson("/health", decode);

  expect(fetchMock).toHaveBeenCalledExactlyOnceWith("/api/v1/health", {
    method: "GET",
    body: undefined,
    credentials: "same-origin",
    redirect: "error",
    headers: { Accept: "application/json" },
  });
  expect(decode).toHaveBeenCalledExactlyOnceWith(body);
  expect(result).toBe("disponible");
});

it("transmet le statut, le code et le message d’une erreur API", async () => {
  const body = { code: "not_found", message: "Route inconnue." };
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(Response.json(body, { status: 404 })),
  );

  await expect(getJson("/inconnue", vi.fn())).rejects.toMatchObject({
    status: 404,
    ...body,
  });
});

it("ignore le refus tardif d’une ancienne session et signale celui de la session courante", async () => {
  const expired = vi.fn();
  const unsubscribe = onSessionExpired(expired);
  const fetchMock = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetchMock);
  const denied = () => new Response(null, { status: 401 });
  try {
    let finish!: (response: Response) => void;
    fetchMock.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const previous = request("/courrier");
    fetchMock.mockResolvedValueOnce(Response.json({}));
    await request("/session", "POST", {});
    finish(denied());
    await expect(previous).rejects.toMatchObject({ status: 401 });
    fetchMock.mockResolvedValueOnce(denied());
    await expect(request("/session", "POST", {})).rejects.toMatchObject({
      status: 401,
    });
    expect(expired).not.toHaveBeenCalled();
    fetchMock.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const current = request("/courrier", "POST", {});
    fetchMock.mockResolvedValueOnce(Response.json({}));
    await request("/session");
    fetchMock.mockResolvedValueOnce(Response.json({}));
    await request("/brouillons", "POST", {});
    finish(denied());
    await expect(current).rejects.toMatchObject({
      status: 401,
    });
    expect(expired).toHaveBeenCalledTimes(1);
  } finally {
    unsubscribe();
  }
});

it.each([
  null,
  "indisponible",
  {},
  { code: 42, message: "Erreur" },
  { code: "erreur", message: 42 },
])(
  "fournit une erreur HTTP générique pour un corps invalide (%j)",
  async (body) => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(Response.json(body, { status: 503 })),
    );

    await expect(getJson("/health", vi.fn())).rejects.toMatchObject({
      status: 503,
      code: "http_error",
      message: "Erreur HTTP 503.",
    });
  },
);
