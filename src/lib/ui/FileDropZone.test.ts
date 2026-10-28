import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import FileDropZone from "./FileDropZone.svelte";

describe("FileDropZone", () => {
  it("calls onPick from the button (OS file dialog)", async () => {
    const onPick = vi.fn();
    render(FileDropZone, { label: "Attach files", onPick, onDrop: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Attach files" }));
    expect(onPick).toHaveBeenCalledOnce();
  });

  it("highlights while dragging and passes dropped files", async () => {
    const onDrop = vi.fn();
    render(FileDropZone, { label: "Attach files", onPick: vi.fn(), onDrop });
    const zone = screen.getByTestId("drop-zone");
    const file = new File(["x"], "spec.pdf", { type: "application/pdf" });
    await fireEvent.dragEnter(zone, { dataTransfer: { files: [file], types: ["Files"] } });
    expect(zone).toHaveAttribute("data-dragging", "true");
    await fireEvent.drop(zone, { dataTransfer: { files: [file], types: ["Files"] } });
    expect(zone).toHaveAttribute("data-dragging", "false");
    expect(onDrop).toHaveBeenCalledWith([file]);
  });

  it("has no a11y violations", async () => {
    const { container } = render(FileDropZone, {
      label: "Attach files",
      onPick: vi.fn(),
      onDrop: vi.fn(),
    });
    await expectNoA11yViolations(container);
  });
});
