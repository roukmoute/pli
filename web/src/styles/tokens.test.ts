// @vitest-environment node
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

const css = readFileSync(join(import.meta.dirname, "tokens.css"), "utf8");
const couleurs = new Map(
  [...css.matchAll(/--([\w-]+):\s*#([\da-f]{3,6});/gi)].map(([, cle, hex]) => [
    cle,
    hex,
  ]),
);

function luminance(cle: string) {
  const valeur = couleurs.get(cle);
  if (!valeur) throw new Error(`Couleur absente du thème : ${cle}`);
  const hex =
    valeur.length === 3 ? [...valeur].map((c) => c + c).join("") : valeur;
  const [r, g, b] = [0, 2, 4].map((offset) => {
    const canal = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return canal <= 0.04045 ? canal / 12.92 : ((canal + 0.055) / 1.055) ** 2.4;
  });
  return r * 0.2126 + g * 0.7152 + b * 0.0722;
}

it("respecte les seuils AA sur le fond, la surface de lecture et les commandes", () => {
  const controles: [string, string, number][] = [
    ["sur-commande", "commande", 4.5],
    ["sur-commande", "commande-survol", 4.5],
    ["tri", "fond-tri", 4.5],
  ];
  for (const fond of ["surface", "fond"]) {
    for (const texte of [
      "texte",
      "texte-secondaire",
      "commande",
      "tri",
      "erreur",
    ])
      controles.push([texte, fond, 4.5]);
    for (const trait of ["focus", "bord-controle"])
      controles.push([trait, fond, 3]);
  }
  for (const [premier, second, minimum] of controles) {
    const a = luminance(premier);
    const b = luminance(second);
    expect(
      (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05),
      `${premier} sur ${second}`,
    ).toBeGreaterThanOrEqual(minimum);
  }
});
