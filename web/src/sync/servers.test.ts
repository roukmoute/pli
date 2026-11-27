import { expect, it, vi } from "vitest";
import { listServers } from "./servers";

it.each([
  [422, null],
  [422, { code: "invalid_profile", message: "Port invalide." }],
  [422, { diagnostic: {}, server: { id: 7 } }],
  [503, { diagnostic: {}, server: null }],
] as const)("propage l’erreur HTTP %s du catalogue", async (status, body) => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(Response.json(body, { status })),
  );
  await expect(listServers("test")).rejects.toMatchObject({ status });
});

it("signale une réponse de succès illisible", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("<html>")));
  await expect(listServers("test")).rejects.toThrow(
    "Réponse d’administration illisible.",
  );
});
