import { expect, it, vi } from "vitest";
import { lireSession } from "./session";

const session = { account_id: 2, address: "bob@recette.test", expires_at: 100 };
it.each([
  null,
  "session",
  {},
  { ...session, account_id: "1" },
  { ...session, address: false },
  { ...session, expires_at: "demain" },
])("rejette les sessions illisibles (%j)", async (body) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(body)));
  await expect(lireSession()).rejects.toThrow("Session illisible.");
});
