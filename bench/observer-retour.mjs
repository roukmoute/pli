export async function observerRequetes(page) {
  await page.addInitScript(() => {
    const lire = window.fetch.bind(window);
    window.fetch = (...arguments_) => {
      window.requetesPendantLeRetour?.push(performance.now());
      return lire(...arguments_);
    };
  });
}

export async function retour(bouton) {
  return bouton.evaluate((element) => new Promise((resoudre, rejeter) => {
    const requetes = [];
    window.requetesPendantLeRetour = requetes;
    const clic = performance.now();
    element.click();
    function verifier() {
      const zone = document.querySelector("main")?.getBoundingClientRect();
      const visible = zone && Array.from(document.querySelectorAll("main [data-fil]")).some((el) => {
        const r = el.getBoundingClientRect();
        return r.height > 0 && r.bottom > zone.top && r.top < zone.bottom;
      });
      if (visible) {
        // À la frame suivante, le navigateur a eu l’occasion de peindre la première ligne.
        requestAnimationFrame((rendu) => {
          delete window.requetesPendantLeRetour;
          resoudre({
            clic, rendu_contenu: rendu, origine_utc: new Date(performance.timeOrigin).toISOString(),
            requetes_avant_rendu: requetes.filter((debut) => debut < rendu).length,
          });
        });
      } else if (performance.now() - clic > 2000) {
        delete window.requetesPendantLeRetour;
        rejeter(new Error("La liste ne présente aucune ligne visible."));
      } else {
        requestAnimationFrame(verifier);
      }
    }
    requestAnimationFrame(verifier);
  }));
}
