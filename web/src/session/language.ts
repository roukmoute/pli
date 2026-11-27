export const labels = {
  fr: {
    language: "Langue",
    title: "Connexion",
    address: "Adresse de messagerie",
    password: "Secret de messagerie",
    open: "Se connecter",
    close: "Se déconnecter",
    inbox: "Réception",
    retry: "Réessayer",
    loading: "Ouverture de Pli…",
    invalid: "Adresse ou secret de messagerie incorrect.",
    offline:
      "Le service de messagerie est indisponible. Réessayez dans un instant.",
    expired: "Votre session a expiré. Reconnectez-vous.",
    senders:
      "Vous retrouverez les demandes de tri dans « Nouveaux expéditeurs ».",
  },
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
