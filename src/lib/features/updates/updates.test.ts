import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import { checkAtStartup, resetUpdates, updates } from "./updates.svelte";

let checks = 0;
let offer: unknown = null;

beforeEach(() => {
  checks = 0;
  offer = null;
  mockIPC((cmd) => {
    if (cmd === "check_for_update") {
      checks++;
      if (offer === "offline") throw { code: "update.check_failed", params: {} };
      return offer;
    }
  });
});

afterEach(() => {
  clearMocks();
  resetUpdates();
});

describe("checkAtStartup", () => {
  it("does not look when the user turned it off", async () => {
    await checkAtStartup({ ...DEFAULT_SETTINGS, checkUpdates: false });
    expect(checks).toBe(0);
    expect(updates.available).toBeNull();
  });

  it("remembers a newer version", async () => {
    offer = { version: "1.0.0", notes: null, date: null };
    await checkAtStartup(DEFAULT_SETTINGS);
    expect(checks).toBe(1);
    expect(updates.available?.version).toBe("1.0.0");
  });

  it("stays quiet when offline", async () => {
    offer = "offline";
    await expect(checkAtStartup(DEFAULT_SETTINGS)).resolves.toBeUndefined();
    expect(updates.available).toBeNull();
  });
});
