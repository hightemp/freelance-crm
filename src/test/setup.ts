import "@testing-library/jest-dom/vitest";

// jsdom has no layout, so no ResizeObserver; Svelte's size bindings need it.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
