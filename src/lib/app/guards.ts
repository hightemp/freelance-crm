// Disables WebView behaviours that make the app feel like a web page
// (FR-UI-5): reload, print, find, zoom, native context menu, dragging links
// out, native validation bubbles.

export interface GuardOptions {
  /** Keeps reload shortcuts working during development. */
  dev: boolean;
  /** Ctrl/Cmd+F opens the app's own search. */
  onFind: () => void;
  /** F1 opens the help. */
  onHelp: () => void;
  /** Ctrl/Cmd +/-/0 changes the app's UI scale instead of WebView zoom. */
  onZoom: (step: -1 | 0 | 1) => void;
  /** Right click in a text field shows the app's cut/copy/paste menu. */
  onEditableContextMenu: (at: { x: number; y: number; target: HTMLElement }) => void;
}

const EDITABLE = 'input, textarea, [contenteditable=""], [contenteditable="true"]';

function isReload(e: KeyboardEvent, mod: boolean) {
  return e.key === "F5" || (mod && e.key.toLowerCase() === "r");
}

export function installBrowserGuards(options: GuardOptions): () => void {
  function onkeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    if (isReload(e, mod)) {
      if (!options.dev) e.preventDefault();
    } else if (mod && k === "p") {
      e.preventDefault();
    } else if (mod && k === "f") {
      e.preventDefault();
      options.onFind();
    } else if (e.key === "F1") {
      e.preventDefault();
      options.onHelp();
    } else if (mod && (k === "=" || k === "+")) {
      e.preventDefault();
      options.onZoom(1);
    } else if (mod && k === "-") {
      e.preventDefault();
      options.onZoom(-1);
    } else if (mod && k === "0") {
      e.preventDefault();
      options.onZoom(0);
    } else if ((e.altKey && (k === "arrowleft" || k === "arrowright")) || k.startsWith("browser")) {
      e.preventDefault();
    }
  }

  function onwheel(e: WheelEvent) {
    if (e.ctrlKey) e.preventDefault();
  }

  // Bubble phase: components with their own context menu prevent default first.
  function oncontextmenu(e: MouseEvent) {
    if (e.defaultPrevented) return;
    e.preventDefault();
    const editable = (e.target as Element | null)?.closest?.(EDITABLE);
    if (editable instanceof HTMLElement) {
      options.onEditableContextMenu({ x: e.clientX, y: e.clientY, target: editable });
    }
  }

  function ondragstart(e: DragEvent) {
    const el = e.target as Element | null;
    if (el?.closest?.("a[href], img")) e.preventDefault();
  }

  function oninvalid(e: Event) {
    e.preventDefault();
  }

  const listeners: [string, EventListener, AddEventListenerOptions][] = [
    ["keydown", onkeydown as EventListener, { capture: true }],
    ["wheel", onwheel as EventListener, { passive: false }],
    ["contextmenu", oncontextmenu as EventListener, {}],
    ["dragstart", ondragstart as EventListener, { capture: true }],
    // `invalid` does not bubble, so it is caught in the capture phase.
    ["invalid", oninvalid, { capture: true }],
  ];
  for (const [type, fn, opts] of listeners) window.addEventListener(type, fn, opts);
  return () => {
    for (const [type, fn, opts] of listeners) window.removeEventListener(type, fn, opts);
  };
}
