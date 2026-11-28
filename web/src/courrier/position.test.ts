import { expect, it, vi } from "vitest";
import type { Repere } from "../sync";
import { memoriserAncre, restaurerPosition } from "./position";

function mesurer(element: Element, top: number, bottom: number) {
  vi.spyOn(element, "getBoundingClientRect").mockImplementation(() =>
    DOMRect.fromRect({ y: top, height: bottom - top }),
  );
}

function liste() {
  const zone = document.createElement("main");
  zone.innerHTML = `<section data-section="nouveaux"><button data-fil="arrivée"></button></section>
    <section data-section="deja_vus"><button data-fil="atelier"></button><button data-fil="livre"></button></section>`;
  const [arrivee, atelier, livre] = zone.querySelectorAll("button");
  mesurer(zone, 60, 600);
  mesurer(arrivee, 22, 60);
  mesurer(atelier, 49, 94);
  mesurer(livre, 94, 136);
  const repere: Repere = {
    defilement: 120,
    section: "nouveaux",
    ordre: "date",
    filtre: "tous",
  };
  return { zone, arrivee, atelier, livre, repere };
}

it("mémorise la première ligne encore visible et son décalage signé dans la zone", () => {
  const { zone, repere, arrivee } = liste();
  memoriserAncre(zone, repere);
  expect(repere.ancre).toEqual({ fil: "atelier", decalage: -11 });
  expect(repere.section).toBe("deja_vus");
  mesurer(arrivee, 60, 98);
  memoriserAncre(zone, repere);
  expect(repere.ancre).toEqual({ fil: "arrivée", decalage: 0 });
  expect(repere.section).toBe("nouveaux");
  zone.replaceChildren();
  memoriserAncre(zone, repere);
  expect(repere.ancre).toEqual({ fil: "arrivée", decalage: 0 });
});

it("corrige le défilement après insertion d’une ligne et rend le focus sans déplacer la page", () => {
  const { zone, atelier, livre, repere } = liste();
  memoriserAncre(zone, repere);
  repere.fil = "livre";
  mesurer(atelier, 75, 124);
  const focus = vi.spyOn(livre, "focus");
  restaurerPosition(zone, repere);
  expect(zone.scrollTop).toBe(146);
  expect(focus).toHaveBeenCalledWith({ preventScroll: true });
});

it("retrouve la section lorsque le fil ancré a disparu et accepte une section vidée", () => {
  const { zone, atelier, livre, repere } = liste();
  memoriserAncre(zone, repere);
  repere.fil = "atelier";
  atelier.remove();
  const focus = vi.spyOn(livre, "focus");
  restaurerPosition(zone, repere);
  expect(zone.scrollTop).toBe(165);
  expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  livre.remove();
  restaurerPosition(zone, repere);
  expect(zone.scrollTop).toBe(120);
  expect(focus).toHaveBeenCalledTimes(1);
});

it("se replace sur la voisine de la section d’origine quand la lettre ouverte a changé de section", () => {
  const { zone, atelier, livre, arrivee, repere } = liste();
  const ancre = { fil: "atelier", decalage: -11 };
  repere.fil = "atelier";
  repere.ancre = ancre;
  repere.lecture = {
    fil: "atelier",
    section: "deja_vus",
    ordre: ["atelier", "disparue", "livre"],
  };
  arrivee.parentElement?.append(atelier);
  const voisine = vi.spyOn(livre, "focus");
  const ouverte = vi.spyOn(atelier, "focus");
  restaurerPosition(zone, repere);
  expect(voisine).toHaveBeenCalledWith({ preventScroll: true });
  expect(ouverte).not.toHaveBeenCalled();
  expect(zone.scrollTop).toBe(165);
  expect(repere.lecture).toBeUndefined();
  repere.lecture = {
    fil: "atelier",
    section: "deja_vus",
    ordre: ["atelier", "livre"],
  };
  livre.remove();
  restaurerPosition(zone, repere);
  expect(ouverte).toHaveBeenCalledWith({ preventScroll: true });
});
