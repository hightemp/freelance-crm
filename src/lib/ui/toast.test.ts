import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ToastHost from "./ToastHost.svelte";
import { resetToasts, toast } from "./toast.svelte";

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  resetToasts();
  vi.useRealTimers();
});

describe("toast", () => {
  it("announces messages in a polite live region", async () => {
    render(ToastHost);
    toast.show({ message: "Client saved" });
    await vi.advanceTimersByTimeAsync(0);
    const region = screen.getByRole("region", { name: "Notifications" });
    expect(region).toHaveTextContent("Client saved");
    expect(screen.getByRole("status")).toHaveTextContent("Client saved");
  });

  it("uses an alert role for errors", async () => {
    render(ToastHost);
    toast.show({ message: "Save failed", kind: "error" });
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.getByRole("alert")).toHaveTextContent("Save failed");
  });

  it("auto-dismisses after the duration", async () => {
    render(ToastHost);
    toast.show({ message: "Saved", duration: 3000 });
    await vi.advanceTimersByTimeAsync(2999);
    expect(screen.queryByText("Saved")).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(1);
    expect(screen.queryByText("Saved")).not.toBeInTheDocument();
  });

  it("can be dismissed manually", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(ToastHost);
    toast.show({ message: "Saved", duration: 0 });
    await vi.advanceTimersByTimeAsync(0);
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByText("Saved")).not.toBeInTheDocument();
  });

  it("shows at most 3 toasts and queues the rest", async () => {
    render(ToastHost);
    for (const n of [1, 2, 3, 4]) toast.show({ message: `T${n}`, duration: 1000 * n });
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.queryByText("T4")).not.toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(1000);
    expect(screen.queryByText("T1")).not.toBeInTheDocument();
    expect(screen.getByText("T4")).toBeInTheDocument();
  });
});
