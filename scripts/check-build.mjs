// Fails if dev-only code (the UI kit showcase, FR-UI-7) leaked into the
// production build. Run after `pnpm build`.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const MARKER = "ui-kit-showcase";
const root = new URL("../build", import.meta.url).pathname;

function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const leaks = files(root).filter((file) => readFileSync(file, "utf8").includes(MARKER));
if (leaks.length > 0) {
  console.error(`Dev-only showcase found in production build:\n  ${leaks.join("\n  ")}`);
  process.exit(1);
}
console.log("build ok: no dev-only code");
