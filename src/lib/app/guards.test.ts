import { fireEvent } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { installBrowserGuards } from "./guards";

let cleanup: (() => void) | undefined;
afterEach(() => {
  cleanup?.();
  document.body.innerHTML = "";
});

function install(dev = false) {
  const handlers = {
    onFind: vi.fn(),
    onHelp: vi.fn(),
    onZoom: vi.fn(),
    onEditableContextMenu: vi.fn(),
  };
  cleanup = installBrowserGuards({ dev, ...handlers });
  return handlers;
}

function key(init: KeyboardEventInit, target: EventTarget = document.body) {
  const event = new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

describe("installBrowserGuards", () => {
  it.each([
    [{ key: "F5" }],
    [{ key: "r", ctrlKey: true }],
    [{ key: "R", ctrlKey: true, shiftKey: true }],
    [{ key: "r", metaKey: true }],
    [{ key: "p", ctrlKey: true }],
    [{ key: "ArrowLeft", altKey: true }],
    [{ key: "BrowserBack" }],
  ])("blocks %j in production", (init) => {
    install();
    expect(key(init).defaultPrevented).toBe(true);
  });

  it("allows reload in dev mode", () => {
    install(true);
    expect(key({ key: "F5" }).defaultPrevented).toBe(false);
    expect(key({ key: "r", ctrlKey: true }).defaultPrevented).toBe(false);
  });

  it("replaces Ctrl+F with the app search", () => {
    const { onFind } = install();
    expect(key({ key: "f", ctrlKey: true }).defaultPrevented).toBe(true);
    expect(onFind).toHaveBeenCalledOnce();
  });

  it("opens the help with F1", () => {
    const { onHelp } = install();
    expect(key({ key: "F1" }).defaultPrevented).toBe(true);
    expect(onHelp).toHaveBeenCalledOnce();
  });

  it.each([
    [{ key: "=", ctrlKey: true }, 1],
    [{ key: "+", ctrlKey: true, shiftKey: true }, 1],
    [{ key: "-", ctrlKey: true }, -1],
    [{ key: "0", ctrlKey: true }, 0],
  ])("routes %j to app zoom (%d)", (init, step) => {
    const { onZoom } = install();
    expect(key(init).defaultPrevented).toBe(true);
    expect(onZoom).toHaveBeenCalledWith(step);
  });

  it("blocks Ctrl+wheel zoom", () => {
    install();
    const event = new WheelEvent("wheel", { ctrlKey: true, cancelable: true, bubbles: true });
    document.body.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it("leaves ordinary keys alone", () => {
    install();
    expect(key({ key: "a" }).defaultPrevented).toBe(false);
    expect(key({ key: "c", ctrlKey: true }).defaultPrevented).toBe(false);
  });

  it("suppresses the native context menu outside editable fields", async () => {
    const { onEditableContextMenu } = install();
    const div = document.body.appendChild(document.createElement("div"));
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    div.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(onEditableContextMenu).not.toHaveBeenCalled();
  });

  it("opens the app edit menu in editable fields", async () => {
    const { onEditableContextMenu } = install();
    const input = document.body.appendChild(document.createElement("input"));
    await fireEvent.contextMenu(input, { clientX: 10, clientY: 20 });
    expect(onEditableContextMenu).toHaveBeenCalledWith({ x: 10, y: 20, target: input });
  });

  it("lets components handle their own context menu", () => {
    const { onEditableContextMenu } = install();
    const div = document.body.appendChild(document.createElement("div"));
    div.addEventListener("contextmenu", (e) => e.preventDefault());
    div.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    expect(onEditableContextMenu).not.toHaveBeenCalled();
  });

  it("prevents dragging links and images out of the window", () => {
    install();
    const link = document.body.appendChild(document.createElement("a"));
    link.href = "https://example.com";
    const event = new Event("dragstart", { bubbles: true, cancelable: true });
    link.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it("suppresses native validation bubbles", () => {
    install();
    const input = document.body.appendChild(document.createElement("input"));
    input.required = true;
    const event = new Event("invalid", { cancelable: true });
    input.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it("cleanup removes all listeners", () => {
    install();
    cleanup?.();
    cleanup = undefined;
    expect(key({ key: "F5" }).defaultPrevented).toBe(false);
  });
});
