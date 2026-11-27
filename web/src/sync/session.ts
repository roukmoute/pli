import { getJson, request } from "./api";

export interface Session {
  account_id: number;
  address: string;
  expires_at: number;
}

function decode(body: unknown): Session {
  if (
    typeof body !== "object" ||
    body === null ||
    !("account_id" in body) ||
    typeof body.account_id !== "number" ||
    !("address" in body) ||
    typeof body.address !== "string" ||
    !("expires_at" in body) ||
    typeof body.expires_at !== "number"
  )
    throw new Error("Session illisible.");
  return {
    account_id: body.account_id,
    address: body.address,
    expires_at: body.expires_at,
  };
}

export function lireSession(): Promise<Session> {
  return getJson("/session", decode);
}

export async function ouvrirSession(
  address: string,
  password: string,
): Promise<Session> {
  const response = await request("/session", "POST", { address, password });
  return decode(await response.json());
}

export async function fermerSession(): Promise<void> {
  await request("/session", "DELETE");
}
