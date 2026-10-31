// README screenshots in English and Russian: `pnpm screenshots`.
// The first run builds the e2e app, the second reuses it.
import { spawnSync } from "node:child_process";

for (const [i, lang] of ["en", "ru"].entries()) {
  const run = spawnSync("pnpm", ["exec", "wdio", "run", "e2e/wdio.screenshots.conf.ts"], {
    stdio: "inherit",
    shell: process.platform === "win32",
    env: {
      ...process.env,
      SCREENSHOTS_LANG: lang,
      ...(i > 0 ? { E2E_SKIP_BUILD: "1" } : {}),
    },
  });
  if (run.status !== 0) process.exit(run.status ?? 1);
}
