import type { Contenu } from "../sync/lecture";

export function documentMessage(
  contenu: Contenu,
  distantes: boolean,
  images: ReadonlyMap<number, string>,
) {
  // Un template reste inerte pendant la résolution des références du message.
  const modele = document.createElement("template");
  modele.innerHTML = contenu.html;
  for (const image of modele.content.querySelectorAll<HTMLImageElement>(
    "img[src]",
  )) {
    const identifiant = /^cid:pli-(\d+)$/.exec(image.src);
    image.removeAttribute("src");
    if (!identifiant) continue;
    const ref = contenu.images[Number(identifiant[1])];
    if (ref?.type === "integree") {
      const donnees = images.get(ref.partie);
      if (donnees) image.setAttribute("src", donnees);
    } else if (
      distantes &&
      ref?.type === "distante" &&
      /^https?:\/\//i.test(ref.url)
    ) {
      image.setAttribute("referrerpolicy", "no-referrer");
      image.setAttribute("src", ref.url);
    }
  }
  const sources = distantes ? "data: https: http:" : "data:";
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${sources}; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'"><meta name="referrer" content="no-referrer"><style>html{color-scheme:light}body{margin:0;color:#242237;background:white;font:1rem/1.55 system-ui,sans-serif;overflow-wrap:anywhere}img{max-width:100%;height:auto}blockquote{border-inline-start:3px solid #dddbe6;margin-inline:0;padding-inline-start:1em;color:#565267}pre{white-space:pre-wrap}table{max-width:100%}a{color:#46368c}</style></head><body>${modele.innerHTML}</body></html>`;
}
