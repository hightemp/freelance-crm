// End-to-end tests against the real app via the embedded WebDriver server
// (@wdio/tauri-service + tauri-plugin-wdio-webdriver, `e2e` cargo feature).
// Works on Linux, Windows and macOS. Run with `pnpm e2e`.
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const binary = join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "freelance-crm.exe" : "freelance-crm",
);

// Each run gets a throwaway data directory; the user's database is never touched.
const dataDir = mkdtempSync(join(tmpdir(), "freelance-crm-e2e-"));
process.env.FREELANCE_CRM_DATA_DIR = dataDir;
// The WebDriver server listens on localhost; never send it through a proxy.
process.env.NO_PROXY = process.env.no_proxy = "127.0.0.1,localhost";

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,
  services: [["@wdio/tauri-service", { appBinaryPath: binary, driverProvider: "embedded" }]],
  capabilities: [{ browserName: "tauri", "tauri:options": { application: binary } }],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 60_000 },

  onPrepare() {
    if (process.env.E2E_SKIP_BUILD) return;
    const build = spawnSync(
      "pnpm",
      // tauri.e2e.conf.json enables the global Tauri API and the wdio
      // capability for this build only.
      [
        "tauri",
        "build",
        "--debug",
        "--no-bundle",
        "--features",
        "e2e",
        "--config",
        "src-tauri/tauri.e2e.conf.json",
      ],
      {
        cwd: root,
        stdio: "inherit",
        shell: process.platform === "win32",
        // Includes the WebdriverIO bridge in the frontend (src/hooks.client.ts).
        env: { ...process.env, VITE_E2E: "1" },
      },
    );
    if (build.status !== 0) throw new Error("tauri build failed");
  },

  // A failed test leaves a screenshot and the page's address and text in
  // e2e/failures (uploaded by CI).
  async afterTest(test, _context, { passed }) {
    if (passed) return;
    const dir = join(root, "e2e", "failures");
    mkdirSync(dir, { recursive: true });
    const name = test.title.replace(/[^\w-]+/g, "-").slice(0, 80);
    try {
      await browser.saveScreenshot(join(dir, `${name}.png`));
      const page = await browser.execute(
        () => `${location.href}\n\n${document.body?.innerText ?? "(no body)"}`,
      );
      writeFileSync(join(dir, `${name}.txt`), page);
    } catch (error) {
      writeFileSync(join(dir, `${name}.txt`), `could not capture the page: ${error}`);
    }
  },

  onComplete() {
    // Windows keeps the database locked for a moment after the app exits.
    try {
      rmSync(dataDir, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
    } catch (error) {
      console.warn(`could not remove ${dataDir}:`, error);
    }
  },
};
