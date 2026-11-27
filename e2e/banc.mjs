import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { test as playwright } from "@playwright/test";

const racine = fileURLToPath(new URL("../", import.meta.url));
const binaire = resolve(racine, process.env.CARGO_TARGET_DIR || "server/target", "release/webmail-server");
export const alice = "alice@recette.test";
export const secret = "alice-test";

function demarrer(commande, arguments_, env = process.env) {
  const enfant = spawn(commande, arguments_, { cwd: racine, env, stdio: ["ignore", "pipe", "inherit"] });
  const lignes = createInterface({ input: enfant.stdout })[Symbol.asyncIterator]();
  const fin = new Promise((resoudre) => {
    enfant.once("error", (e) => resoudre(e.message));
    enfant.once("exit", (code, signal) => resoudre(signal || code));
  });
  async function ligne(motif) {
    const delai = setTimeout(() => enfant.kill(), 25000);
    try {
      while (true) {
        const { done, value } = await lignes.next();
        if (done) throw Error("Le processus du banc s’est arrêté en attendant " + motif);
        const trouve = value.match(motif);
        if (trouve) return trouve;
      }
    } finally {
      clearTimeout(delai);
    }
  }
  async function arreter() {
    enfant.kill();
    const delai = setTimeout(() => enfant.kill("SIGKILL"), 4000);
    try { await fin; } finally { clearTimeout(delai); }
  }
  return { ligne, fin, arreter };
}

export async function livrer(action) {
  const envoi = demarrer("python3", ["-B", "e2e/courrier.py", action]);
  const statut = await envoi.fin;
  if (statut !== 0) throw Error("Échec du dépôt SMTP sur le banc. " + statut);
}

export const test = playwright.extend({
  banc: async ({ request }, utiliser) => {
    const dossier = await mkdtemp(join(tmpdir(), "pli-e2e-"));
    const cle = randomBytes(32).toString("hex");
    const jeton = randomBytes(32).toString("hex");
    const proxy = demarrer("python3", ["-B", "e2e/courrier.py", "historique", dossier]);
    let serveur;
    let port = 0;
    async function lancer() {
      serveur = demarrer(binaire, [], {
        ...process.env, NO_COLOR: "1", WEBMAIL_LISTEN: "127.0.0.1:" + port,
        WEBMAIL_WEB_DIR: join(racine, "web/dist"), WEBMAIL_DATA_DIR: dossier,
        WEBMAIL_SECRET_KEY: cle, WEBMAIL_ADMIN_TOKEN: jeton, WEBMAIL_ALLOW_TEST_PROFILE: "1",
      });
      port = Number((await serveur.ligne(/address=127\.0\.0\.1:(\d+)/))[1]);
    }
    try {
      const imap = Number((await proxy.ligne(/^(\d+) alice@recette\.test$/))[1]);
      await lancer();
      const base = "http://127.0.0.1:" + port;
      const url = base + "/api/admin/v1/servers";
      const headers = { Authorization: "Bearer " + jeton };
      const catalogue = await (await request.get(url, { headers })).json();
      const profil = catalogue.presets.find((p) => p.local_test);
      profil.imap.port = imap;
      profil.smtp.port = Number(process.env.PLI_BENCH_SMTPS_PORT);
      const resultat = await request.post(url, {
        headers, data: { profile: profil, credentials: { username: alice, password: secret } },
      });
      if (!resultat.ok()) throw Error("Création du profil fictif, HTTP " + resultat.status());
      await utiliser({
        base,
        lotSuspendu: () => proxy.ligne(/^lot suspendu$/),
        async redemarrer() {
          await serveur.arreter();
          await lancer();
        },
      });
    } finally {
      try {
        if (serveur) await serveur.arreter();
      } finally {
        await proxy.arreter();
        await rm(dossier, { recursive: true, force: true });
      }
    }
  },
});
