export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
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
    return new ApiError(status, body.code, body.message);
  }
  return new ApiError(status, "http_error", `Erreur HTTP ${status}.`);
}

export async function getJson<T>(
  path: `/${string}`,
  decode: (body: unknown) => T,
): Promise<T> {
  const response = await fetch(`/api/v1${path}`, {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    throw responseError(
      response.status,
      await response.json().catch(() => null),
    );
  }
  return decode(await response.json());
}
