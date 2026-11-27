import { responseError } from "./api";

export type Protocol = "imap" | "smtp";
export type TlsMode = "implicit" | "starttls";

export interface Endpoint {
  host: string;
  port: number;
  tls: TlsMode;
}

export interface Profile {
  name: string;
  imap: Endpoint;
  smtp: Endpoint;
  domains: string[];
  active: boolean;
  local_test: boolean;
}

export interface Server extends Profile {
  id: number;
}

export interface Catalog {
  servers: Server[];
  presets: Profile[];
}

export const stages = [
  "resolution",
  "connection",
  "tls",
  "authentication",
] as const;
export type Stage = (typeof stages)[number];
export type Diagnostic = Record<
  Protocol,
  Record<Stage, "ok" | "failed" | "skipped">
>;

export interface Candidate {
  profile: Profile;
  credentials: { username: string; password: string };
}

interface SaveResult {
  diagnostic: Diagnostic;
  server: Server | null;
}

async function request<T>(
  token: string,
  path: string,
  method: string,
  candidate?: Candidate,
): Promise<T> {
  const headers = new Headers({
    Accept: "application/json",
    Authorization: `Bearer ${token}`,
  });
  if (candidate) headers.set("Content-Type", "application/json");
  const response = await fetch(`/api/admin/v1/servers${path}`, {
    method,
    headers,
    credentials: "omit",
    redirect: "error",
    body: JSON.stringify(candidate),
  });
  const body = await response.json().catch(() => null);
  // A failed save still carries the two protocol reports for the form.
  if (
    !response.ok &&
    !(response.status === 422 && body?.diagnostic && body.server === null)
  ) {
    throw responseError(response.status, body);
  }
  if (body === null) throw new Error("Réponse d’administration illisible.");
  return body;
}

export function listServers(token: string) {
  return request<Catalog>(token, "", "GET");
}

export function testServer(token: string, candidate: Candidate) {
  return request<Diagnostic>(token, "/test", "POST", candidate);
}

export function saveServer(
  token: string,
  id: number | undefined,
  candidate: Candidate,
) {
  return id === undefined
    ? request<SaveResult>(token, "", "POST", candidate)
    : request<SaveResult>(token, `/${id}`, "PUT", candidate);
}
