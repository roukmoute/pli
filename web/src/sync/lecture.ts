import { request } from "./api";
import { comparerFils } from "./projection-tri";
import { ressource } from "./ressource";
import type { PageFils, Vue } from "./vues";

export interface Entete {
  id: string;
  objet: string;
  nom: string | null;
  adresse: string | null;
  destinataires: { to?: Contact[]; cc?: Contact[]; bcc?: Contact[] };
  date: number;
  non_lu: boolean;
}
interface Contact {
  address: string;
  name: string | null;
}
export type ImageMessage =
  | { type: "integree"; partie: number }
  | { type: "distante"; url: string };
export interface Contenu {
  html: string;
  images: ImageMessage[];
  pieces: { nom: string | null; mime: string; taille: number }[];
}
export type Corps =
  | { etat: "disponible"; contenu: Contenu }
  | { etat: "indisponible" };
export interface Lecture {
  fil: string;
  revision: number;
  non_lus: Partial<Record<Vue, boolean>>;
}

export function appliquerLecture(
  page: PageFils,
  vue: Vue,
  lecture: Lecture,
): PageFils {
  const nonLu = lecture.non_lus[vue];
  if (nonLu === undefined || page.revision > lecture.revision) return page;
  const fils = [...page.nouveaux, ...page.deja_vus].map((f) =>
    f.id === lecture.fil ? { ...f, non_lu: nonLu } : f,
  );
  return {
    ...page,
    nouveaux: fils.filter((f) => f.non_lu).sort(comparerFils),
    deja_vus: fils.filter((f) => !f.non_lu).sort(comparerFils),
  };
}

function cache<T>(lire: (id: string) => Promise<T>) {
  const entries = new Map<string, ReturnType<typeof ressource<T>>>();
  return (id: string) => {
    const existante = entries.get(id);
    if (existante) entries.delete(id);
    const entree = existante || ressource(() => lire(id));
    entries.set(id, entree);
    if (entries.size > 40)
      entries.delete(entries.keys().next().value as string);
    return entree;
  };
}

export function creerLectures(confirmer: (lecture: Lecture) => void) {
  const fil = cache<Entete[]>(async (id) =>
    (await request(`/threads/${encodeURIComponent(id)}`)).json(),
  );
  const corps = cache<Corps>(async (id) =>
    (await request(`/messages/${encodeURIComponent(id)}`)).json(),
  );
  const enCours = new Map<string, Promise<void>>();
  const revisions = new Map<string, number>();
  function ouvrir(id: string, filId: string) {
    const attente = enCours.get(id);
    if (attente) return attente;
    const ecriture = request(`/messages/${encodeURIComponent(id)}/open`, "POST")
      .then(async (r) => {
        const lecture: Lecture = await r.json();
        if (lecture.fil !== filId) throw Error("Fil de lecture incohérent.");
        fil(filId).corriger((liste) =>
          liste.map((m) => (m.id === id ? { ...m, non_lu: false } : m)),
        );
        const derniere = revisions.get(filId);
        if (derniere === undefined || lecture.revision > derniere) {
          revisions.set(filId, lecture.revision);
          confirmer(lecture);
        }
      })
      .finally(() => enCours.delete(id));
    enCours.set(id, ecriture);
    return ecriture;
  }
  return { fil, corps, ouvrir };
}
export type Lectures = ReturnType<typeof creerLectures>;

export async function imageIntegree(
  id: string,
  partie: number,
  signal?: AbortSignal,
): Promise<string> {
  const response = await request(
    `/messages/${encodeURIComponent(id)}/images/${partie}`,
    "GET",
    undefined,
    signal,
  );
  const blob = await response.blob();
  if (
    !["image/png", "image/jpeg", "image/gif", "image/webp"].includes(blob.type)
  )
    throw Error("Format d’image indisponible.");
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}
