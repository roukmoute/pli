import { For } from "solid-js";
import { type Diagnostic as Report, stages } from "../sync";
import styles from "./admin.module.css";

const labels = {
  resolution: "Résolution DNS",
  connection: "Connexion TCP",
  tls: "Négociation TLS",
  authentication: "Authentification",
};
const statuses = { ok: "Réussi", failed: "Échec", skipped: "Non testé" };

export default function Diagnostic(props: { report: Report }) {
  return (
    <table class={styles.results}>
      <caption>Diagnostic de connexion</caption>
      <thead>
        <tr>
          <th scope="col">Étape</th>
          <th scope="col">IMAP</th>
          <th scope="col">SMTP</th>
        </tr>
      </thead>
      <tbody>
        <For each={stages}>
          {(stage) => (
            <tr>
              <th scope="row">{labels[stage]}</th>
              <For each={["imap", "smtp"] as const}>
                {(protocol) => (
                  <td data-status={props.report[protocol][stage]}>
                    {statuses[props.report[protocol][stage]]}
                  </td>
                )}
              </For>
            </tr>
          )}
        </For>
      </tbody>
    </table>
  );
}
