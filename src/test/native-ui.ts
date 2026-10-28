// Static check for FR-UI-8: app code must use the UI kit (src/lib/ui)
// instead of native browser controls, dialogs and tooltips.

export interface Violation {
  rule: string;
  line: number;
}

const NATIVE_INPUT_TYPES =
  "date|time|datetime-local|month|week|color|range|file|number|checkbox|radio";

/** Blanks out comments, keeping newlines so line numbers stay correct. */
function stripComments(source: string): string {
  const blank = (match: string) => match.replace(/[^\n]/g, " ");
  return source
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(
      /(^|[^:"'`])\/\/.*$/gm,
      (match, prefix: string) => prefix + blank(match.slice(prefix.length)),
    );
}

function importedFromKit(source: string, name: string): boolean {
  const re = new RegExp(`import\\s*\\{[^}]*\\b${name}\\b[^}]*\\}\\s*from\\s*["']\\$lib/ui`);
  return re.test(source);
}

export function findNativeUi(source: string): Violation[] {
  const code = stripComments(source);
  const lineAt = (index: number) => code.slice(0, index).split("\n").length;
  const violations: Violation[] = [];
  const check = (re: RegExp, rule: (m: RegExpExecArray) => string) => {
    for (const match of code.matchAll(
      new RegExp(re.source, re.flags.includes("g") ? re.flags : `${re.flags}g`),
    )) {
      violations.push({ rule: rule(match), line: lineAt(match.index ?? 0) });
    }
  };

  check(/<select\b/, () => "select");
  check(/<details\b/, () => "details");
  check(/<dialog\b/, () => "dialog");
  check(
    new RegExp(`<input\\b[^>]*?\\btype\\s*=\\s*["']?(${NATIVE_INPUT_TYPES})\\b`),
    (m) => `input[type="${m[1]}"]`,
  );
  // `title` on HTML elements (lowercase tags); components may have a title prop.
  check(/<[a-z][\w-]*\b[^>]*?\stitle\s*=/, () => "title attribute");
  check(/\bwindow\.(alert|confirm|prompt)\s*\(/, (m) => `window.${m[1]}()`);
  for (const name of ["alert", "confirm", "prompt"]) {
    if (name !== "alert" && importedFromKit(source, name)) continue;
    check(new RegExp(`(?<![\\w.$])${name}\\s*\\(`), () => `${name}()`);
  }
  return violations;
}
