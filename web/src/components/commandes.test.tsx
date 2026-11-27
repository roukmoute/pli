import { fireEvent, render } from "@solidjs/testing-library";
import { createSignal, Errored } from "solid-js";
import { expect, it, vi } from "vitest";
import { Icone, type NomIcone } from "../icons/Icone";
import { Avatar } from "./Avatar";
import { Bouton, BoutonIcone } from "./Bouton";

it("conserve les attributs natifs, le focus et le type du bouton", () => {
  const action = vi.fn();
  const vue = render(() => (
    <form>
      <Bouton onClick={action}>Annuler</Bouton>
      <Bouton type="submit" disabled>
        Envoyer
      </Bouton>
      <BoutonIcone icone="archive" libelle="Archiver le fil" onClick={action} />
    </form>
  ));
  const annuler = vue.getByRole("button", {
    name: "Annuler",
  }) as HTMLButtonElement;
  expect(annuler.type).toBe("button");
  annuler.focus();
  expect(document.activeElement).toBe(annuler);
  const envoyer = vue.getByRole("button", {
    name: "Envoyer",
  }) as HTMLButtonElement;
  expect(envoyer.type).toBe("submit");
  expect(envoyer.disabled).toBe(true);
  const archiver = vue.getByRole("button", { name: "Archiver le fil" });
  expect(archiver.hasAttribute("icone")).toBe(false);
  expect(archiver.hasAttribute("libelle")).toBe(false);
  fireEvent.click(archiver);
  expect(action).toHaveBeenCalledOnce();
});

const noms: NomIcone[] = [
  "menu",
  "recherche",
  "ecrire",
  "etoile",
  "pieceJointe",
  "repondre",
  "transferer",
  "archive",
  "corbeille",
  "dossier",
  "plus",
];

it("rend les onze pictogrammes avec un tracé distinct et un nom accessible", () => {
  const vue = render(() => (
    <div>
      {noms.map((nom) => (
        <Icone nom={nom} libelle={nom} />
      ))}
    </div>
  ));
  const traces = noms.map((nom) =>
    vue
      .getByRole("img", { name: nom })
      .querySelector("path")
      ?.getAttribute("d"),
  );
  expect(traces.every(Boolean)).toBe(true);
  expect(new Set(traces).size).toBe(noms.length);
});

it("exige un nom accessible pour chaque icône", async () => {
  const vue = render(() => (
    <Errored fallback={(erreur) => <p>{String(erreur())}</p>}>
      <Icone nom="menu" libelle="  " />
    </Errored>
  ));
  expect(
    await vue.findByText(/Une icône exige un nom accessible/),
  ).toBeDefined();
});

it("présente des initiales lisibles pour un nom, une adresse ou un expéditeur inconnu", () => {
  const vue = render(() => (
    <div>
      <Avatar nom="  élise  de Beaulieu " />
      <Avatar nom="noa@recette.test" />
      <Avatar nom=" " />
    </div>
  ));
  expect(vue.getByRole("img", { name: "élise de Beaulieu" }).textContent).toBe(
    "ÉB",
  );
  expect(vue.getByRole("img", { name: "noa@recette.test" }).textContent).toBe(
    "N",
  );
  expect(vue.getByRole("img", { name: "Expéditeur inconnu" }).textContent).toBe(
    "?",
  );
});

it("retrouve les initiales après une erreur et affiche la nouvelle image du compte", async () => {
  const [src, changer] = createSignal("/portrait-1.png");
  const vue = render(() => <Avatar nom="Élise Noël" src={src()} />);
  const avatar = vue.getByRole("img", { name: "Élise Noël" });
  const photo = avatar.querySelector("img");
  expect(photo?.getAttribute("src")).toBe("/portrait-1.png");
  if (!photo) throw new Error("Photo attendue");
  fireEvent.error(photo);
  await vi.waitFor(() => expect(avatar.textContent).toBe("ÉN"));
  changer("/portrait-2.png");
  await vi.waitFor(() =>
    expect(avatar.querySelector("img")?.getAttribute("src")).toBe(
      "/portrait-2.png",
    ),
  );
});
