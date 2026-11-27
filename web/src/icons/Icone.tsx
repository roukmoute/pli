const traces = {
  reception: "M3 13l3-8h12l3 8v6H3Zm0 0h5l2 3h4l2-3h5",
  flux: "M4 18c1.1 0 2 .9 2 2M4 12c4.4 0 8 3.6 8 8M4 5c8.3 0 15 6.7 15 15",
  justificatifs: "M6 3h12v18l-3-2-3 2-3-2-3 2Zm3 5h6m-6 4h6",
  plusTard: "M21 12a9 9 0 1 1-18 0 9 9 0 1 1 18 0Zm-9-5v5l4 2",
  deCote: "M6 4h12v17l-6-4-6 4Z",
  fermer: "m6 6 12 12M18 6 6 18",
  menu: "M3 6h18M3 12h14M3 18h18",
  recherche: "M16.5 10a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0ZM15 15l6 6",
  ecrire: "M4 16l-1 5 5-1L20 8l-4-4ZM13.5 6.5l4 4",
  etoile:
    "m12 2.8 2.8 5.8 6.4.9-4.6 4.5 1.1 6.4-5.7-3-5.7 3 1.1-6.4-4.6-4.5 6.4-.9Z",
  pieceJointe:
    "m8 13 6.5-6.5a2.1 2.1 0 0 1 3 3L10 17a4.2 4.2 0 0 1-6-6l7-7a5.7 5.7 0 0 1 8 8l-7.5 7.5",
  repondre: "m9 4-6 6 6 6M3.5 10H12c5 0 8 3 9 8",
  transferer: "m15 4 6 6-6 6M20.5 10H12c-5 0-8 3-9 8",
  archive: "M3 4h18v5H3ZM5 9v11h14V9M9 12h6",
  corbeille: "M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7M14 10v7",
  dossier: "M3 7V5h6l2 3h10v11H3Z",
  plus: "M5 11a1 1 0 1 0 0 2 1 1 0 0 0 0-2Zm7 0a1 1 0 1 0 0 2 1 1 0 0 0 0-2Zm7 0a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z",
} as const;

export type NomIcone = keyof typeof traces;

export function Icone(props: { nom: NomIcone; libelle: string }) {
  const nomAccessible = () => {
    const libelle = props.libelle.trim();
    if (!libelle) throw new Error("Une icône exige un nom accessible.");
    return libelle;
  };
  return (
    <svg
      role="img"
      aria-label={nomAccessible()}
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <title>{props.libelle}</title>
      <path d={traces[props.nom]} />
    </svg>
  );
}
