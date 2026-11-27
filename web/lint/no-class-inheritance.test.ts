import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempDisposableSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, it } from "vitest";

function lint(filename: string, source: string) {
  const web = join(import.meta.dirname, "..");
  using directory = mkdtempDisposableSync(join(tmpdir(), "pli-lint-"));
  for (const name of ["package.json", "biome.json", "tsconfig.json"]) {
    copyFileSync(join(web, name), join(directory.path, name));
  }
  for (const name of ["node_modules", "lint"]) {
    symlinkSync(join(web, name), join(directory.path, name), "junction");
  }
  const src = join(directory.path, "src");
  mkdirSync(src);
  writeFileSync(join(src, filename), `${source}\n`);
  return spawnSync("npm run lint", {
    cwd: directory.path,
    encoding: "utf8",
    shell: true,
  });
}

it.each([
  ["exemple.ts", "class B {}\nexport class A extends B {}"],
  ["exemple.tsx", "export const A = class extends TypeError {};"],
])("refuse une classe dérivée dans %s", (filename, source) => {
  const result = lint(filename, source);
  expect(result.status, result.stderr).toBe(1);
  expect(result.stderr).toContain("Seules les classes d’erreur");
});

it("autorise les erreurs et les contraintes de types", () => {
  const result = lint(
    "exemple.ts",
    `export class MonErreur extends Error {}
interface Message {
  id: string;
}
export interface Brouillon extends Message {
  texte: string;
}
export type Id<T extends Message> = T["id"];`,
  );
  expect(result.status, result.stderr).toBe(0);
});
