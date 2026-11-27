import { connexionFr } from "../i18n/fr";

export const labels = {
  fr: connexionFr,
  en: {
    language: "Language",
    title: "Sign in",
    address: "Email address",
    password: "Email password",
    open: "Sign in",
    close: "Sign out",
    inbox: "Inbox",
    retry: "Try again",
    loading: "Opening Pli…",
    invalid: "Incorrect email address or password.",
    offline: "The mail service is unavailable. Please try again in a moment.",
    expired: "Your session has expired. Please sign in again.",
    senders: "You’ll find requests to screen mail under “New senders”.",
  },
};

export type Language = keyof typeof labels;
export type Labels = typeof labels.fr;
