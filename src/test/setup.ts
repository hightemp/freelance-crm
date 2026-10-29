import "@testing-library/jest-dom/vitest";

// jsdom has no layout, so no ResizeObserver; Svelte's size bindings need it.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// Nor IntersectionObserver (infinite scroll); tests that need it stub it.
globalThis.IntersectionObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return [];
  }
} as unknown as typeof IntersectionObserver;
