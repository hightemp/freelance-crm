import { autoUpdate, computePosition, flip, offset, shift, type Placement } from "@floating-ui/dom";
import type { Attachment } from "svelte/attachments";

/**
 * Runs `fn` after the current effect flush. Focus set during mount would be
 * lost when `portal` moves the subtree, so initial focus must wait for it.
 */
export function afterPortal(fn: () => void): void {
  queueMicrotask(fn);
}

/** Moves the element to <body> so overlays escape clipping and stacking contexts. */
export const portal: Attachment<HTMLElement> = (node) => {
  node.dataset.overlay = "";
  document.body.appendChild(node);
  return () => node.remove();
};

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "textarea:not([disabled])",
  "select:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

export function focusableIn(container: Element): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(FOCUSABLE)];
}

/**
 * Keeps Tab focus inside the element while mounted, focuses
 * `[data-autofocus]` (or the first focusable) on mount and restores focus
 * to the previously focused element on unmount.
 */
export const trapFocus: Attachment<HTMLElement> = (node) => {
  const previous = document.activeElement as HTMLElement | null;
  afterPortal(() => {
    const initial = node.querySelector<HTMLElement>("[data-autofocus]") ?? focusableIn(node)[0];
    (initial ?? node).focus();
  });

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "Tab") return;
    const items = focusableIn(node);
    if (items.length === 0) return event.preventDefault();
    const first = items[0];
    const last = items[items.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener("keydown", onkeydown);
  return () => {
    node.removeEventListener("keydown", onkeydown);
    if (previous?.isConnected) previous.focus();
  };
};

/**
 * Calls `onOutside` on pointerdown outside the element and the `ignore`
 * elements. Overlays opened from inside it (a select's list in a popover)
 * are portaled after it and count as inside.
 */
export function clickOutside(
  onOutside: () => void,
  ignore: () => (Element | null | undefined)[] = () => [],
): Attachment<HTMLElement> {
  return (node) => {
    function onpointerdown(event: PointerEvent) {
      const target = event.target as Node;
      if (node.contains(target)) return;
      if (ignore().some((el) => el?.contains(target))) return;
      const overlay = target instanceof Element ? target.closest("[data-overlay]") : null;
      if (overlay && node.compareDocumentPosition(overlay) & Node.DOCUMENT_POSITION_FOLLOWING)
        return;
      onOutside();
    }
    document.addEventListener("pointerdown", onpointerdown, true);
    return () => document.removeEventListener("pointerdown", onpointerdown, true);
  };
}

/** A reference for floating-ui: an element or a fixed point (context menus). */
export type Anchor = Element | { x: number; y: number };

function virtual(anchor: Anchor) {
  if (anchor instanceof Element) return anchor;
  return {
    getBoundingClientRect: () => new DOMRect(anchor.x, anchor.y, 0, 0),
  };
}

/** Positions the floating element next to the anchor and keeps it there. */
export function floating(
  anchor: () => Anchor | undefined,
  placement: Placement = "bottom-start",
): Attachment<HTMLElement> {
  return (node) => {
    const reference = anchor();
    if (!reference) return;
    node.style.position = "fixed";
    const ref = virtual(reference);
    const update = () =>
      computePosition(ref, node, {
        strategy: "fixed",
        placement,
        middleware: [offset(4), flip(), shift({ padding: 8 })],
      }).then(({ x, y }) => {
        node.style.left = `${x}px`;
        node.style.top = `${y}px`;
      });
    if (reference instanceof Element) return autoUpdate(reference, node, update);
    void update();
  };
}
