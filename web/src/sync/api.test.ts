import { expect, it, vi } from "vitest";
import { getJson } from "./api";

it("appelle l’API v1 et renvoie le résultat du décodeur", async () => {
  const body = { status: "ok" };
  const fetchMock = vi
    .fn<typeof fetch>()
    .mockResolvedValue(Response.json(body));
  vi.stubGlobal("fetch", fetchMock);
  const decode = vi.fn((_body: unknown) => "disponible");

  const result = await getJson("/health", decode);

  expect(fetchMock).toHaveBeenCalledExactlyOnceWith("/api/v1/health", {
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
