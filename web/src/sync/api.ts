export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly details?: unknown,
  ) {
    super(message);
  }
}

export function responseError(status: number, body: unknown): ApiError {
  if (
    typeof body === "object" &&
    body !== null &&
    "code" in body &&
    typeof body.code === "string" &&
    "message" in body &&
    typeof body.message === "string"
  ) {
    return new ApiError(status, body.code, body.message, body);
  }
  return new ApiError(status, "http_error", `Erreur HTTP ${status}.`, body);
}

const sessionEvents = new EventTarget();
let sessionRevision = 0;

export function onSessionExpired(callback: () => void) {
  sessionEvents.addEventListener("expired", callback);
  return () => sessionEvents.removeEventListener("expired", callback);
}

export async function request(
  path: `/${string}`,
  method = "GET",
  payload?: unknown,
  signal?: AbortSignal,
): Promise<Response> {
  const revision = sessionRevision;
  const headers: Record<string, string> = { Accept: "application/json" };
  if (payload !== undefined) headers["Content-Type"] = "application/json";
  const response = await fetch(`/api/v1${path}`, {
    method,
    headers,
    body: JSON.stringify(payload),
    credentials: "same-origin",
    redirect: "error",
    ...(signal && { signal }),
  });
  if (!response.ok) {
    if (
      response.status === 401 &&
      revision === sessionRevision &&
      !(path === "/session" && method === "POST")
    ) {
      sessionEvents.dispatchEvent(new Event("expired"));
    }
    throw responseError(
      response.status,
      await response.json().catch(() => null),
    );
  }
  if (path === "/session" && method !== "GET") sessionRevision++;
  return response;
}

export async function getJson<T>(
  path: `/${string}`,
  decode: (body: unknown) => T,
): Promise<T> {
  const response = await request(path);
  return decode(await response.json());
}
