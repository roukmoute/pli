import { createMemo, Errored, Loading } from "solid-js";
import styles from "./App.module.css";
import Admin from "./admin/Admin";
import { getServiceHealth } from "./sync";

function ServiceStatus() {
  const health = createMemo(getServiceHealth);
  return (
    <p class={styles.available}>
      Serveur disponible <code>{health().status}</code>
    </p>
  );
}

export default function App() {
  if (["/admin", "/admin/"].includes(window.location.pathname))
    return <Admin />;
  return (
    <main class={styles.page}>
      <h1 class={styles.brand}>Pli</h1>
      <section class={styles.panel} aria-labelledby="status-title">
        <h2 id="status-title">État du service</h2>
        <div aria-live="polite">
          <Errored
            fallback={(_, retry) => (
              <>
                <p>Le service est indisponible.</p>
                <button type="button" onClick={retry}>
                  Réessayer
                </button>
              </>
            )}
          >
            <Loading fallback={<p>Vérification en cours…</p>}>
              <ServiceStatus />
            </Loading>
          </Errored>
        </div>
      </section>
    </main>
  );
}
