import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import DocumentSettings from "./DocumentSettings.svelte";

const PNG = "data:image/png;base64,iVBORw0KGgo=";
let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let images: Record<string, string | null> = {};

beforeEach(() => {
  calls = [];
  images = { logo: PNG, signature: null, stamp: null };
  appSettings.value = { ...DEFAULT_SETTINGS };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "profile_image":
        return images[a.kind as string];
      case "plugin:dialog|open":
        return "/home/me/stamp.png";
      case "set_profile_image":
        if (a.path === "/home/me/broken.png") throw { code: "image.unsupported", params: {} };
        images[a.kind as string] = PNG;
        return null;
      case "delete_profile_image":
        images[a.kind as string] = null;
        return null;
      case "save_settings":
        return a.settings;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

function setup() {
  render(DocumentSettings);
  render(ToastHost);
  return userEvent.setup();
}

describe("DocumentSettings", () => {
  it("shows set images and placeholders", async () => {
    setup();
    expect(await screen.findByRole("img", { name: "Logo" })).toHaveAttribute("src", PNG);
    const stamp = screen.getByRole("group", { name: "Stamp" });
    expect(within(stamp).getByText("Not set")).toBeInTheDocument();
  });

  it("uploads and removes an image", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: "Choose Stamp…" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "set_profile_image")?.args).toEqual({
        kind: "stamp",
        path: "/home/me/stamp.png",
      }),
    );
    expect(await screen.findByRole("img", { name: "Stamp" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Remove Logo" }));
    await waitFor(() => expect(screen.queryByRole("img", { name: "Logo" })).toBeNull());
  });

  it("toggles the signature default", async () => {
    const user = setup();
    await user.click(
      screen.getByRole("switch", { name: "Signature and stamp on invoices by default" }),
    );
    await waitFor(() => expect(appSettings.value.invoiceShowSignature).toBe(true));
  });

  it("has no a11y violations", async () => {
    setup();
    await screen.findByRole("img", { name: "Logo" });
    await expectNoA11yViolations(document.body);
  });
});
