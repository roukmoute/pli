export function reponseRetenue<T = Response>() {
  let resolve!: (reponse: T) => void;
  const promise = new Promise<T>((terminer) => {
    resolve = terminer;
  });
  return { promise, resolve };
}
