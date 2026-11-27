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
