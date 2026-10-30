// Smoke test of a built app binary (T-12.5): it starts on a throwaway data
// folder, creates its database and keeps running. Release builds have no
// WebDriver, so this is what can be checked on them.
//
//   node scripts/smoke.mjs <path to the app binary>
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const binary = process.argv[2];
if (!binary || !existsSync(binary)) {
  console.error(`usage: node scripts/smoke.mjs <app binary> (got ${binary ?? "nothing"})`);
  process.exit(2);
}

const data = mkdtempSync(join(tmpdir(), "freelance-crm-smoke-"));
const app = spawn(binary, [], {
  env: { ...process.env, FREELANCE_CRM_DATA_DIR: data },
  stdio: "inherit",
});
let exited = null;
app.on("exit", (code, signal) => (exited = { code, signal }));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const fail = (message) => {
  console.error(`smoke: ${message}`);
  app.kill();
  rmSync(data, { recursive: true, force: true });
  process.exit(1);
};

const deadline = Date.now() + 60_000;
while (!existsSync(join(data, "crm.db"))) {
  if (exited) fail(`the app exited early: ${JSON.stringify(exited)}`);
  if (Date.now() > deadline) fail("no database after 60 s");
  await sleep(250);
}
// Still running a few seconds later: no crash right after start.
await sleep(5_000);
if (exited) fail(`the app exited: ${JSON.stringify(exited)}`);

app.kill();
await sleep(500);
rmSync(data, { recursive: true, force: true });
console.log("smoke: ok");
