import { createSignal, Match, onSettled, Show, Switch } from "solid-js";
import { Bouton } from "../components/Bouton";
import Courrier from "../courrier/Courrier";
import {
  ApiError,
  fermerSession,
  lireSession,
  onSessionExpired,
  type Session,
} from "../sync";
import Login from "./Login";
import { type Language, labels } from "./language";
import styles from "./mail.module.css";

type Page =
  | { kind: "loading" | "offline" }
  | { kind: "login"; expired: boolean }
  | { kind: "inbox"; session: Session };

export default function Mail() {
  const [page, setPage] = createSignal<Page>({ kind: "loading" });
  const [language, setLanguage] = createSignal<Language>("fr");
  const text = () => labels[language()];
  const [closeFailed, setCloseFailed] = createSignal<boolean>();
  let timer: number | undefined;
  let pendingRead: Promise<Session> | undefined;

  function login(expired: boolean) {
    pendingRead = undefined;
    window.clearTimeout(timer);
    setPage({ kind: "login", expired });
  }

  function checkDeadline(session: Session) {
    const remaining = session.expires_at * 1000 - Date.now();
    if (remaining <= 0) login(true);
    else
      timer = window.setTimeout(
        () => checkDeadline(session),
        Math.min(remaining, 2_147_483_647),
      );
  }

  function inbox(session: Session) {
    pendingRead = undefined;
    window.clearTimeout(timer);
    setCloseFailed(false);
    setPage({ kind: "inbox", session });
    checkDeadline(session);
  }

  async function restore() {
    const attempt = lireSession();
    pendingRead = attempt;
    try {
      const session = await attempt;
      if (attempt === pendingRead) inbox(session);
    } catch (cause) {
      if (attempt !== pendingRead) return;
      if (cause instanceof ApiError && cause.status === 401) login(false);
      else setPage({ kind: "offline" });
    }
  }

  onSettled(() => {
    const unsubscribe = onSessionExpired(() => {
      if (page().kind === "inbox") login(true);
    });
    const revalidate = () => {
      if (page().kind === "inbox") void restore();
    };
    window.addEventListener("focus", revalidate);
    window.addEventListener("pageshow", revalidate);
    void restore();
    return () => {
      pendingRead = undefined;
      window.clearTimeout(timer);
      window.removeEventListener("focus", revalidate);
      window.removeEventListener("pageshow", revalidate);
      unsubscribe();
    };
  });

  async function close(button: HTMLButtonElement) {
    if (button.disabled) return;
    button.disabled = true;
    try {
      await fermerSession();
      login(false);
    } catch {
      setCloseFailed(true);
      button.disabled = false;
    }
  }

  const loginPage = () => {
    const p = page();
    return p.kind === "login" && p;
  };
  const activeSession = () => {
    const p = page();
    return p.kind === "inbox" ? p.session : undefined;
  };
  return (
    <Show
      when={activeSession()?.account_id}
      keyed
      fallback={
        <main class={styles.page}>
          <section class={styles.card}>
            <header class={styles.header}>
              <span class={styles.brand}>Pli</span>
              <label class={styles.language}>
                {text().language}
                <select
                  value={language()}
                  onChange={(e) => {
                    const value = e.currentTarget.value === "en" ? "en" : "fr";
                    document.documentElement.lang = value;
                    setLanguage(value);
                  }}
                >
                  <option value="fr" lang="fr">
                    Français
                  </option>
                  <option value="en" lang="en">
                    English
                  </option>
                </select>
              </label>
            </header>
            <Switch fallback={<p role="status">{text().loading}</p>}>
              <Match when={loginPage()} keyed>
                {(p) => (
                  <Login text={text()} expired={p.expired} onOpen={inbox} />
                )}
              </Match>
              <Match when={page().kind === "offline"}>
                <p role="alert">{text().offline}</p>
                <Bouton type="button" onClick={() => void restore()}>
                  {text().retry}
                </Bouton>
              </Match>
            </Switch>
          </section>
        </main>
      }
    >
      {(compte) => (
        <Courrier
          compte={compte}
          adresse={activeSession()?.address ?? ""}
          textes={text()}
          fermer={close}
          echec={Boolean(closeFailed())}
        />
      )}
    </Show>
  );
}
