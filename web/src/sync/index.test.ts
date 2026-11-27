import { expect, it, vi } from "vitest";
import { getServiceHealth } from "./index";

it("décode l’état disponible du service", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(Response.json({ status: "ok" })),
  );

  await expect(getServiceHealth()).resolves.toEqual({ status: "ok" });
});

it.each([null, "ok", {}, { status: "inconnu" }])(
  "rejette un état de service invalide (%j)",
  async (body) => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(body)));

    await expect(getServiceHealth()).rejects.toThrow(
      "Le serveur a renvoyé un état inconnu.",
    );
  },
);
