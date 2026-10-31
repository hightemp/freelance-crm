import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { resetToasts } from "$lib/ui/toast.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import UpdateNotice from "./UpdateNotice.svelte";
import { resetUpdates, updates } from "./updates.svelte";

let installs = 0;
let installFails = false;

beforeEach(() => {
  installs = 0;
  installFails = false;
  mockIPC((cmd) => {
    if (cmd === "install_update") {
      installs++;
      if (installFails) throw { code: "update.install_failed", params: {} };
      return null;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetUpdates();
  resetToasts();
});

const offer = () => (updates.available = { version: "1.0.0", notes: "Faster search", date: null });

describe("UpdateNotice", () => {
  it("shows nothing without an update", () => {
    render(UpdateNotice);
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("offers the update and installs it", async () => {
    offer();
    const { container } = render(UpdateNotice);
    await userEvent.click(screen.getByRole("button", { name: "Update to 1.0.0" }));
    const dialog = screen.getByRole("dialog", { name: "Version 1.0.0 is available" });
    expect(within(dialog).getByText("Faster search")).toBeInTheDocument();
    await expectNoA11yViolations(container.ownerDocument.body);

    await userEvent.click(within(dialog).getByRole("button", { name: "Install and restart" }));
    expect(installs).toBe(1);
  });

  it("can be put off", async () => {
    offer();
    render(UpdateNotice);
    await userEvent.click(screen.getByRole("button", { name: "Update to 1.0.0" }));
    await userEvent.click(screen.getByRole("button", { name: "Later" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(installs).toBe(0);
  });

  it("reports a failed installation", async () => {
    offer();
    installFails = true;
    render(UpdateNotice);
    render(ToastHost);
    await userEvent.click(screen.getByRole("button", { name: "Update to 1.0.0" }));
    await userEvent.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(await screen.findByText("Could not install the update")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
