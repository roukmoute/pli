import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { fileURLToPath } from "node:url";
import { connecter, ouvrirNavigateur, tailles } from "./navigateur.mjs";

const base = process.argv[2];
function sonde(fichier, args = []) {
  const chemin = fileURLToPath(new URL(fichier, import.meta.url));
  return execFileSync("python3", [chemin, ...args], {
    encoding: "utf8",
  }).trim();
}
let distantes = 0;
const photo = readFileSync(new URL("./fixtures/timbre.png", import.meta.url));
const temoin = createServer((req, res) => {
  assert.equal(req.url, "/jardin.png");
  distantes++;
  res.writeHead(200, {
    "content-type": "image/png",
    "cache-control": "no-store",
  });
  res.end(photo);
});
await new Promise((resolve) => temoin.listen(0, "127.0.0.1", resolve));
temoin.unref();
const adresse = sonde("./lecture.py", [String(temoin.address().port)]);
const nonLus = () => Number(sonde("./screener.py", ["--unread", adresse]));
const browser = await ouvrirNavigateur();
async function api(page, chemin) {
  return page.evaluate(async (chemin) => {
    const r = await fetch("/api/v1" + chemin);
    return { status: r.status, body: await r.json() };
  }, chemin);
}
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(6500);
  await connecter(page, base, adresse, adresse);
  const jardin = page.getByRole("button", { name: /Le jardin en images/ });
  const cle = await jardin.getAttribute("data-fil");
  const fil = await api(page, "/threads/" + encodeURIComponent(cle));
  assert.equal(fil.status, 200);
  const id = fil.body.at(-1).id;
  const chemin = "/messages/" + id;
  const avant = nonLus();
  const corps = await api(page, chemin);
  assert.equal(corps.body.etat, "disponible");
  assert(!corps.body.contenu.html.includes("127.0.0.1"));
  const integree = corps.body.contenu.images.find((i) => i.type === "integree");
  assert(integree);
  assert(corps.body.contenu.images.some((i) => i.type === "distante"));
  const deltaPrechargement = avant - nonLus();
  const bob = await browser.newPage();
  await connecter(bob, base, "bob@recette.test", "bob-test");
  const autre = (await api(bob, chemin)).status;
  assert.equal(
    (await api(bob, chemin + "/images/" + integree.partie)).status,
    404,
  );
  assert.equal(
    (await api(bob, "/threads/" + encodeURIComponent(cle))).status,
    404,
  );
  await bob.close();
  const confirmation = page.waitForResponse((r) =>
    r.url().endsWith(chemin + "/open"),
  );
  await jardin.click();
  const reponse = await confirmation;
  assert.equal(reponse.status(), 200, await reponse.text());
  const cadre = page.frameLocator("iframe");
  await cadre.getByText(/Voici le timbre/).waitFor();
  await cadre.locator("img[alt^=Timbre]").evaluate((img) => img.decode());
  assert.equal(
    await cadre.locator("img[alt^=Jardin]").getAttribute("src"),
    null,
  );
  const avantClic = distantes;
  await page
    .getByRole("button", { name: /Afficher les images distantes/ })
    .click();
  await cadre.locator("img[alt^=Jardin][src]").evaluate((img) => img.decode());
  assert(distantes > 0);
  const ouverture = avant - nonLus();
  const retour = page.getByRole("button", { name: /Retour/ });
  const bloque = Promise.withResolvers();
  await page.route("**/api/v1/views/reception", async (route) => {
    await bloque.promise;
    await route.continue();
  });
  await retour.click();
  const ligne = page
    .locator("[data-section=deja_vus]")
    .getByRole("button", { name: /Le jardin en images/ });
  await ligne.waitFor();
  assert(await page.locator("[data-section=nouveaux] button:focus").count());
  bloque.resolve();
  await page.unrouteAll({ behavior: "wait" });
  const plan = page.getByRole("button", { name: /Plan de montage/ });
  await plan.click();
  await cadre.getByText("Le montage est terminé.").waitFor();
  assert.equal(await page.locator("article").count(), 2);
  const premiere = page.locator("article").first();
  const bouton = premiere.getByRole("button");
  const haut = await bouton.evaluate((el) => el.getBoundingClientRect().top);
  await bouton.click();
  await premiere
    .frameLocator("iframe")
    .getByText(/Voici le plan/)
    .waitFor();
  assert(
    Math.abs(
      (await bouton.evaluate((el) => el.getBoundingClientRect().top)) - haut,
    ) <= 1,
  );
  const url = await premiere.locator("a").getAttribute("href");
  await page.goto(new URL(url, base).href);
  await page
    .frameLocator("iframe")
    .getByText(/Voici le plan/)
    .waitFor();
  for (const taille of tailles) {
    await page.setViewportSize(taille);
    assert(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    );
  }
  if (process.env.PLI_CAPTURE_FIL)
    await page.screenshot({ path: process.env.PLI_CAPTURE_FIL });
  await page.getByRole("button", { name: /Retour/ }).click();
  await page.getByRole("button", { name: /Reçu de la librairie/ }).click();
  await page.getByText("recu.pdf").waitFor();
  await page.getByText(/application\/pdf/).waitFor();
  console.log(
    JSON.stringify({
      prechargement: deltaPrechargement,
      ouverture,
      distantes: avantClic,
      autre,
    }),
  );
} finally {
  await browser.close();
  temoin.close();
}
