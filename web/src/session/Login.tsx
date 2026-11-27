import { createSignal, Show } from "solid-js";
import { ApiError, ouvrirSession, type Session } from "../sync";
import type { Labels } from "./language";
import styles from "./mail.module.css";

export default function Login(props: {
  text: Labels;
  expired: boolean;
  onOpen: (session: Session) => void;
}) {
  const [error, setError] = createSignal<"invalid" | "offline">();
  let password!: HTMLInputElement;
  let button!: HTMLButtonElement;

  async function submit(form: HTMLFormElement) {
    // Deux événements Entrée peuvent arriver avant le prochain rendu de Solid.
    if (button.disabled) return;
    button.disabled = true;
    setError(undefined);
    const data = new FormData(form);
    try {
      props.onOpen(
        await ouvrirSession(String(data.get("address")).trim(), password.value),
      );
    } catch (cause) {
      const invalid = cause instanceof ApiError && cause.status === 401;
      setError(invalid ? "invalid" : "offline");
      button.disabled = false;
      if (invalid) {
        password.value = "";
        password.focus();
      } else button.focus();
    }
  }

  return (
    <form
      class={styles.form}
      onSubmit={(e) => {
        e.preventDefault();
        void submit(e.currentTarget);
      }}
    >
      <h1>{props.text.title}</h1>
      <Show when={props.expired}>
        <p role="status">{props.text.expired}</p>
      </Show>
      <label>
        {props.text.address}
        <input
          name="address"
          type="email"
          autocomplete="username"
          autocapitalize="none"
          spellcheck="false"
          required
        />
      </label>
      <label>
        {props.text.password}
        <input
          ref={password}
          name="password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>
      <Show when={error()} keyed>
        {(reason) => <p role="alert">{props.text[reason]}</p>}
      </Show>
      <button ref={button} type="submit">
        {props.text.open}
      </button>
    </form>
  );
}
