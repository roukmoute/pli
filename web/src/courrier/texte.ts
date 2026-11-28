/** Ramène un libellé à sa forme de recherche : sans accents, en minuscules. */
export function normaliser(texte: string) {
  return texte
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLocaleLowerCase("fr")
    .trim();
}
