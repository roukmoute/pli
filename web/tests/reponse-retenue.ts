export function reponseRetenue() {
  let resolve!: (reponse: Response) => void;
  const promise = new Promise<Response>((terminer) => {
    resolve = terminer;
  });
  return { promise, resolve };
}
