import { fr } from "./fr";

export { fr };
export type CleLibelle = keyof typeof fr;
export type Dictionnaire = Readonly<Record<CleLibelle, string>>;

export function t(cle: CleLibelle, dictionnaire: Dictionnaire = fr): string {
  if (!Object.hasOwn(dictionnaire, cle)) {
    throw new Error(`Libellé inconnu : ${cle}`);
  }
  return dictionnaire[cle];
}
