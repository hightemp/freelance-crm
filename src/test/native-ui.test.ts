import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { findNativeUi } from "./native-ui";

describe("findNativeUi", () => {
  it.each([
    ["<select bind:value={x}><option>1</option></select>", "select"],
    ['<input type="date" />', 'input[type="date"]'],
    ["<input type='checkbox'>", 'input[type="checkbox"]'],
    ['<input\n  class="x"\n  type="number"\n/>', 'input[type="number"]'],
    ['<input type="file" multiple />', 'input[type="file"]'],
    ["<details><summary>x</summary></details>", "details"],
    ["<dialog open>x</dialog>", "dialog"],
    ['<button title="Copy">x</button>', "title attribute"],
    ["<script>alert('x')</script>", "alert()"],
    ["window.confirm('sure?')", "window.confirm()"],
    ["if (confirm('sure?')) del();", "confirm()"],
    ["const name = prompt('Name');", "prompt()"],
  ])("flags %j", (source, rule) => {
    expect(findNativeUi(source).map((v) => v.rule)).toContain(rule);
  });

  it.each([
    ['<input type="text" />'],
    ['<input type="email" bind:value={email} />'],
    ["<input />"],
    ['<Dialog title="Edit client" />'],
    ['<TextField label="Title" />'],
    ['import { confirm } from "$lib/ui";\nif (await confirm({ title: "x" })) del();'],
    ['import { confirm, prompt } from "$lib/ui/dialogs.svelte";\nawait prompt({ title: "x" });'],
    ["<svelte:head><title>freelance-crm</title></svelte:head>"],
    ["const confirmed = isConfirmed(x);"],
    ["// alert( is mentioned in a comment"],
  ])("allows %j", (source) => {
    expect(findNativeUi(source)).toEqual([]);
  });

  it("reports line numbers", () => {
    expect(findNativeUi("<div>\n  <select></select>\n</div>")[0].line).toBe(2);
  });
});

const ROOT = join(__dirname, "..");
const ALLOWED = ["lib/ui/", "lib/paraglide/", "lib/api/bindings.ts", "test/native-ui"];

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(svelte|ts)$/.test(name) ? [path] : [];
  });
}

describe("app sources (FR-UI-8)", () => {
  it("use the UI kit instead of native browser controls", () => {
    const violations = sourceFiles(ROOT)
      .map((file) => relative(ROOT, file).replaceAll("\\", "/"))
      .filter((file) => !ALLOWED.some((prefix) => file.startsWith(prefix)))
      .flatMap((file) =>
        findNativeUi(readFileSync(join(ROOT, file), "utf8")).map(
          (v) => `src/${file}:${v.line} ${v.rule}`,
        ),
      );
    expect(violations).toEqual([]);
  });
});
