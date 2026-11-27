import { createSignal, For, Show } from "solid-js";
import layout from "../App.module.css";
import {
  type Catalog,
  listServers,
  type Diagnostic as Report,
  type Server,
} from "../sync";
import styles from "./admin.module.css";
import Diagnostic from "./Diagnostic";
import ProfileForm from "./ProfileForm";

interface Session {
  token: string;
  catalog: Catalog;
}

function Access(props: { onOpen: (session: Session) => void }) {
  const [error, setError] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  async function open(form: HTMLFormElement) {
    if (busy()) return;
    const token = String(new FormData(form).get("token"));
    setError("");
    setBusy(true);
    try {
      props.onOpen({ token, catalog: await listServers(token) });
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : "Le serveur est indisponible.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        void open(event.currentTarget);
      }}
    >
      <h2>Accès administrateur</h2>
      <label>
        Jeton d’administration{" "}
        <input name="token" type="password" autocomplete="off" required />
      </label>
      <button type="submit" disabled={busy()}>
        Ouvrir l’administration
      </button>
      <p role="status">{error()}</p>
    </form>
  );
}

type Editor = { kind: "create" } | { kind: "edit"; server: Server };

function Servers(props: { session: Session; onClose: () => void }) {
  const [servers, setServers] = createSignal(
    () => props.session.catalog.servers,
  );
  const [editor, setEditor] = createSignal<Editor>();
  const [saved, setSaved] = createSignal<Report>();

  function open(edit: Editor) {
    setSaved(undefined);
    setEditor(edit);
  }
  function savedProfile(server: Server, report: Report) {
    setServers((previous) =>
      [...previous.filter((p) => p.id !== server.id), server].sort(
        (a, b) => a.id - b.id,
      ),
    );
    setSaved(report);
    setEditor(undefined);
  }
  return (
    <>
      <Show
        when={editor()}
        keyed
        fallback={
          <>
            <h2>Serveurs de messagerie</h2>
            <Show
              when={servers().length}
              fallback={<p>Aucun serveur configuré.</p>}
            >
              <ul class={styles.servers}>
                <For each={servers()}>
                  {(server) => (
                    <li>
                      <div>
                        <strong>{server.name}</strong> ·{" "}
                        {server.active ? "Actif" : "Inactif"}
                        <div>
                          {[server.imap.host, ...server.domains].join(" · ")}
                        </div>
                      </div>
                      <button
                        class={styles.secondary}
                        type="button"
                        aria-label={`Modifier ${server.name}`}
                        onClick={() => open({ kind: "edit", server })}
                      >
                        Modifier
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <button type="button" onClick={() => open({ kind: "create" })}>
              Ajouter un serveur
            </button>
            <div aria-live="polite">
              <Show when={saved()} keyed>
                {(report) => (
                  <>
                    <p>Profil enregistré.</p>
                    <Diagnostic report={report} />
                  </>
                )}
              </Show>
            </div>
          </>
        }
      >
        {(edit) => (
          <ProfileForm
            token={props.session.token}
            presets={props.session.catalog.presets}
            server={edit.kind === "edit" ? edit.server : undefined}
            onSaved={savedProfile}
            onCancel={() => setEditor(undefined)}
          />
        )}
      </Show>
      <button type="button" class={styles.secondary} onClick={props.onClose}>
        Fermer l’administration
      </button>
    </>
  );
}

export default function Admin() {
  document.title = "Pli · Administration";
  const [session, setSession] = createSignal<Session>();
  return (
    <main class={layout.page}>
      <h1 class={layout.brand}>Administration de Pli</h1>
      <section class={`${layout.panel} ${styles.panel}`}>
        <Show when={session()} keyed fallback={<Access onOpen={setSession} />}>
          {(current) => (
            <Servers session={current} onClose={() => setSession(undefined)} />
          )}
        </Show>
      </section>
    </main>
  );
}
