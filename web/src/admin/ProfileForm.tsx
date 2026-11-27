import { createSignal, For, Show } from "solid-js";
import {
  type Candidate,
  type Endpoint,
  type Profile,
  type Protocol,
  type Diagnostic as Report,
  type Server,
  saveServer,
  testServer,
} from "../sync";
import styles from "./admin.module.css";
import Diagnostic from "./Diagnostic";

function endpointFields(protocol: Protocol, endpoint: Endpoint) {
  return (
    <fieldset class={styles.protocol}>
      <legend>{protocol.toUpperCase()}</legend>
      <label>
        Hôte <input name={`${protocol}-host`} value={endpoint.host} required />
      </label>
      <label>
        Port{" "}
        <input
          name={`${protocol}-port`}
          type="number"
          min="1"
          max="65535"
          value={endpoint.port}
          required
        />
      </label>
      <label>
        Chiffrement{" "}
        <select name={`${protocol}-tls`} value={endpoint.tls}>
          <option value="implicit">TLS implicite</option>
          <option value="starttls">STARTTLS</option>
        </select>
      </label>
    </fieldset>
  );
}

function readCandidate(form: HTMLFormElement, local: boolean): Candidate {
  const data = new FormData(form);
  const text = (key: string) => String(data.get(key) ?? "");
  const endpoint = (key: Protocol): Endpoint => ({
    host: text(`${key}-host`).trim(),
    port: Number(text(`${key}-port`)),
    tls: text(`${key}-tls`) === "starttls" ? "starttls" : "implicit",
  });
  return {
    profile: {
      name: text("name").trim(),
      domains: text("domains")
        .split(/[\s,]+/)
        .filter(Boolean),
      active: data.has("active"),
      local_test: local,
      imap: endpoint("imap"),
      smtp: endpoint("smtp"),
    },
    credentials: {
      username: text("username").trim(),
      password: text("password"),
    },
  };
}

export default function ProfileForm(props: {
  token: string;
  presets: Profile[];
  server?: Server;
  onSaved: (server: Server, report: Report) => void;
  onCancel: () => void;
}) {
  const [template, setTemplate] = createSignal<Profile>(
    () => props.server ?? props.presets[0],
  );
  const [report, setReport] = createSignal<Report>();
  const [message, setMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  function clearResult() {
    setReport(undefined);
    setMessage("");
  }

  async function submit(form: HTMLFormElement, save: boolean) {
    if (busy()) return;
    const candidate = readCandidate(form, template().local_test);
    setBusy(true);
    clearResult();
    try {
      if (save) {
        const result = await saveServer(
          props.token,
          props.server?.id,
          candidate,
        );
        if (result.server) props.onSaved(result.server, result.diagnostic);
        else {
          setReport(result.diagnostic);
          setMessage("Enregistrement refusé. Vérifiez les étapes en échec.");
        }
      } else setReport(await testServer(props.token, candidate));
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Le serveur est indisponible.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <form
      onInput={clearResult}
      onSubmit={(event) => {
        event.preventDefault();
        void submit(
          event.currentTarget,
          event.submitter?.getAttribute("value") === "save",
        );
      }}
    >
      <h2>{props.server ? "Modifier le serveur" : "Ajouter un serveur"}</h2>
      <fieldset class={styles.controls} disabled={busy()}>
        <Show when={!props.server}>
          <label>
            Préréglage{" "}
            <select
              onChange={(event) => {
                setTemplate(props.presets[Number(event.currentTarget.value)]);
                clearResult();
              }}
            >
              <For each={props.presets}>
                {(p, index) => <option value={index()}>{p.name}</option>}
              </For>
            </select>
          </label>
        </Show>
        <Show when={template()} keyed>
          {(p) => (
            <>
              <label>
                Nom du serveur <input name="name" value={p.name} required />
              </label>
              <div class={styles.protocols}>
                {endpointFields("imap", p.imap)}
                {endpointFields("smtp", p.smtp)}
              </div>
              <Show when={p.local_test}>
                <p>Le certificat du banc local n’est pas vérifié.</p>
              </Show>
              <label>
                Domaines associés (séparés par des virgules){" "}
                <input name="domains" value={p.domains.join(", ")} />
              </label>
              <label class={styles.check}>
                <input type="checkbox" name="active" checked={p.active} />{" "}
                Profil actif
              </label>
            </>
          )}
        </Show>
        <fieldset class={styles.protocol}>
          <legend>Identifiants de test</legend>
          <label>
            Identifiant de messagerie{" "}
            <input name="username" autocomplete="off" required />
          </label>
          <label>
            Mot de passe de messagerie{" "}
            <input
              name="password"
              type="password"
              autocomplete="off"
              required
            />
          </label>
        </fieldset>
        <div class={styles.actions}>
          <button type="submit" value="test">
            Tester
          </button>
          <button type="submit" value="save">
            Tester et enregistrer
          </button>
          <button
            type="button"
            class={styles.secondary}
            onClick={props.onCancel}
          >
            Annuler
          </button>
        </div>
      </fieldset>
      <div aria-live="polite">
        <Show when={busy()}>
          <p>Diagnostic en cours…</p>
        </Show>
        <Show when={message()}>
          <p>{message()}</p>
        </Show>
        <Show when={report()} keyed>
          {(value) => <Diagnostic report={value} />}
        </Show>
      </div>
    </form>
  );
}
