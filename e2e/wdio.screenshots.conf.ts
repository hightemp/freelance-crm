// README screenshots (`pnpm screenshots`): the e2e setup with the
// screenshot script instead of the tests.
import { config as e2e } from "./wdio.conf";

export const config: WebdriverIO.Config = {
  ...e2e,
  specs: ["./screenshots/readme.ts"],
  afterTest: undefined,
};
