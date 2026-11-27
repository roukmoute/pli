import { expect, it } from "vitest";
import { fr, t } from "./index";

it("utilise le français par défaut et accepte un catalogue de même forme", () => {
  expect(t("inbox")).toBe("Réception");
  expect(t("moveToJunk")).toBe("Déplacer vers Indésirables");
  expect(t("inbox", { ...fr, inbox: "Inbox" })).toBe("Inbox");
});

it("refuse une clé absente à la compilation et à l’exécution", () => {
  // @ts-expect-error Une faute de clé doit également casser le contrôle TypeScript.
  expect(() => t("nouveauxMessagesInconnu")).toThrow("nouveauxMessagesInconnu");
  // @ts-expect-error Les propriétés héritées ne sont pas des libellés.
  expect(() => t("toString")).toThrow("toString");
});
