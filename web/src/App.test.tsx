import { render } from "@solidjs/testing-library";
import { expect, it, vi } from "vitest";
import App from "./App";
import { getServiceHealth } from "./sync";

vi.mock(import("./sync"), () => ({ getServiceHealth: vi.fn() }));

it("affiche la disponibilité du serveur après la vérification", async () => {
  vi.mocked(getServiceHealth).mockResolvedValue({ status: "ok" });
  const view = render(() => <App />);

  view.getByText("Vérification en cours…");
  const message = await view.findByText("Serveur disponible");
  expect(message.textContent).toBe("Serveur disponible ok");
});
