import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../../test/a11y";
import RecoveryKeySetup from "./RecoveryKeySetup.svelte";

const KEY = [
  "ABCD",
  "EFGH",
  "IJKL",
  "MNOP",
  "QRST",
  "UVWX",
  "YZ23",
  "4567",
  "AAAA",
  "BBBB",
  "CCCC",
  "DDDD",
  "EEEE",
  "FFFF",
].join("-");

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    if (cmd === "plugin:dialog|save") return "/home/me/recovery.txt";
    return null;
  });
});

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

const setup = () => {
  const oncomplete = vi.fn();
  // Groups 3 and 10 (1-based) are asked for.
  render(RecoveryKeySetup, { recoveryKey: KEY, oncomplete, pickGroups: () => [2, 9] });
  return oncomplete;
};

describe("RecoveryKeySetup", () => {
  it("shows the key in groups", () => {
    setup();
    expect(screen.getByRole("heading", { name: "Save your recovery key" })).toBeInTheDocument();
    const key = screen.getByRole("group", { name: "Recovery key" });
    expect(key).toHaveTextContent(KEY.replaceAll("-", " "));
  });

  it("saves the key to a file chosen in the system dialog", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Save to file" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toContain("save_recovery_key_file"));
    expect(calls.find((c) => c.cmd === "save_recovery_key_file")!.args).toEqual({
      path: "/home/me/recovery.txt",
      recoveryKey: KEY,
    });
  });

  it("prints the key", async () => {
    const print = vi.spyOn(window, "print").mockImplementation(() => {});
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Print" }));
    expect(print).toHaveBeenCalledOnce();
  });

  it("does not continue until two groups are typed correctly", async () => {
    const oncomplete = setup();
    await userEvent.click(screen.getByRole("button", { name: "I have saved it" }));
    expect(screen.getByText("Enter groups 3 and 10 of your key.")).toBeInTheDocument();
    await userEvent.type(screen.getByRole("textbox", { name: "Group 3" }), "ijkl");
    await userEvent.type(screen.getByRole("textbox", { name: "Group 10" }), "BBBX");
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.getByRole("alert")).toHaveTextContent("The groups do not match the key");
    expect(oncomplete).not.toHaveBeenCalled();

    await userEvent.clear(screen.getByRole("textbox", { name: "Group 10" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Group 10" }), " bbbb ");
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(oncomplete).toHaveBeenCalledOnce();
  });

  it("can go back to the key", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "I have saved it" }));
    await userEvent.click(screen.getByRole("button", { name: "Show the key again" }));
    expect(screen.getByRole("group", { name: "Recovery key" })).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(RecoveryKeySetup, {
      recoveryKey: KEY,
      oncomplete: vi.fn(),
    });
    await expectNoA11yViolations(container);
  });
});
