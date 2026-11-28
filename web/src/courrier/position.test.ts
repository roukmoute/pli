import { expect, it, vi } from "vitest";
import type { Repere } from "../sync";
import {
  memoriserAncre,
  releverAppuis,
  restaurerPosition,
  retablirAppui,
  viserDebut,
} from "./position";

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

function rangees() {
  const zone = document.createElement("main");
  zone.innerHTML = `<h2>Nouveaux</h2><ul>
    <li><label>Passée</label><button data-fil="passée"></button></li>
    <li><label>Atelier</label><button data-fil="atelier"></button></li>
    <li><label>Livre</label><button data-fil="livre"></button></li>
    <li><label>Suivante</label><button data-fil="suivante"></button></li></ul>`;
  const [passee, atelier, livre, suivante] = zone.querySelectorAll("button");
  mesurer(zone, 60, 600);
  // La première ligne finit au bord haut, la dernière commence au bord bas.
  mesurer(passee, 22, 60);
  mesurer(atelier, 49, 94);
  mesurer(livre, 94, 136);
  mesurer(suivante, 600, 642);
  zone.scrollTop = 120;
  return { zone, passee, atelier, livre, suivante };
}

it("relève les lignes visibles avec leur place et commence par celle sous le pointeur", () => {
  const { zone, livre } = rangees();
  const visibles = [
    { fil: "atelier", decalage: -11 },
    { fil: "livre", decalage: 34 },
  ];
  expect(releverAppuis(zone, null)).toEqual(visibles);
  expect(releverAppuis(zone, undefined)).toEqual(visibles);
  expect(releverAppuis(zone, zone.querySelector("h2"))).toEqual(visibles);
  const portrait = livre.previousElementSibling;
  expect(releverAppuis(zone, portrait)).toEqual([visibles[1], visibles[0]]);
  expect(releverAppuis(zone, livre)).toEqual([visibles[1], visibles[0]]);
  // En haut de liste, l’arrivée pousse les lignes : rien n’est retenu.
  zone.scrollTop = 0;
  expect(releverAppuis(zone, portrait)).toEqual([]);
});

it("replace le premier appui encore présent à l’endroit relevé", () => {
  const { zone, atelier, livre } = rangees();
  const appuis = [
    { fil: "disparue", decalage: 5 },
    { fil: "atelier", decalage: -11 },
    { fil: "livre", decalage: 34 },
  ];
  // Une ligne insérée plus haut a fait descendre l’atelier de 26 pixels.
  mesurer(atelier, 75, 120);
  mesurer(livre, 150, 192);
  retablirAppui(zone, appuis);
  expect(zone.scrollTop).toBe(146);
  retablirAppui(zone, [{ fil: "disparue", decalage: 5 }]);
  retablirAppui(zone, []);
  expect(zone.scrollTop).toBe(146);
});

it("remonte en haut de liste et place le focus sur sa première ligne", () => {
  const { zone, passee, atelier } = rangees();
  const premiere = vi.spyOn(passee, "focus");
  const deuxieme = vi.spyOn(atelier, "focus");
  viserDebut(zone);
  expect(zone.scrollTop).toBe(0);
  expect(premiere).toHaveBeenCalledWith({ preventScroll: true });
  expect(deuxieme).not.toHaveBeenCalled();
  zone.replaceChildren();
  zone.scrollTop = 80;
  viserDebut(zone);
  expect(zone.scrollTop).toBe(0);
});

it("reprend sur la ligne la plus proche d’une section vidée, puis sur le titre d’une liste vide", () => {
  const zone = document.createElement("main");
  zone.innerHTML = `<h1 tabindex="-1">Réception</h1>
    <section data-section="nouveaux"><button data-fil="a"></button><button data-fil="b"></button><button data-fil="c"></button></section>
    <section data-section="deja_vus"><button data-fil="d"></button><button data-fil="e"></button></section>`;
  const [a, b, c, d, e] = Array.from(zone.querySelectorAll("button"));
  const titre = zone.querySelector("h1") as HTMLElement;
  const focus = [a, b, c, d, e, titre].map((cible) => vi.spyOn(cible, "focus"));
  const recu = () => focus.map((espion) => espion.mock.calls.length);
  const repere: Repere = {
    defilement: 0,
    section: "deja_vus",
    ordre: "date",
    filtre: "tous",
    fil: "archivé",
  };
  // « Déjà vus » s’est vidé : la dernière ligne de « Nouveaux » est juste au-dessus.
  d.remove();
  e.remove();
  restaurerPosition(zone, repere);
  expect(recu()).toEqual([0, 0, 1, 0, 0, 0]);
  expect(focus[2]).toHaveBeenCalledWith({ preventScroll: true });
  // « Nouveaux » s’est vidé : la première ligne de « Déjà vus » est juste en dessous.
  zone.querySelector("[data-section=deja_vus]")?.append(d, e);
  for (const ligne of [a, b, c]) ligne.remove();
  repere.section = "nouveaux";
  restaurerPosition(zone, repere);
  expect(recu()).toEqual([0, 0, 1, 1, 0, 0]);
  // Plus aucune ligne : le titre de la liste reçoit le focus.
  d.remove();
  e.remove();
  restaurerPosition(zone, repere);
  expect(recu()).toEqual([0, 0, 1, 1, 0, 1]);
  expect(focus[5]).toHaveBeenCalledWith({ preventScroll: true });
  titre.remove();
  expect(() => restaurerPosition(zone, repere)).not.toThrow();
});
