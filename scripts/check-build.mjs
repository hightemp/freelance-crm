// Fails if dev/test-only code leaked into the production build: the UI kit
// showcase (FR-UI-7) or the WebdriverIO bridge used by e2e tests.
// Run after a regular `pnpm build`.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const MARKERS = ["ui-kit-showcase", "__wdio_original_core__"];
const root = new URL("../build", import.meta.url).pathname;

function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const leaks = files(root).filter((file) => {
  const text = readFileSync(file, "utf8");
  return MARKERS.some((marker) => text.includes(marker));
});
if (leaks.length > 0) {
  console.error(`Dev/test-only code found in production build:\n  ${leaks.join("\n  ")}`);
  process.exit(1);
}
console.log("build ok: no dev-only code");
