import { getJson } from "./api";

export * from "./servers";

export interface ServiceHealth {
  status: "ok";
}

export function getServiceHealth(): Promise<ServiceHealth> {
  return getJson("/health", (body) => {
    if (
      typeof body === "object" &&
      body !== null &&
      "status" in body &&
      body.status === "ok"
    ) {
      return { status: body.status };
    }
    throw new Error("Le serveur a renvoyé un état inconnu.");
  });
}
