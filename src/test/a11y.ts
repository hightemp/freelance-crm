import axe from "axe-core";
import { expect } from "vitest";

/** Runs axe on a rendered container. */
export async function expectNoA11yViolations(container: Element): Promise<void> {
  const result = await axe.run(container, {
    rules: {
      // jsdom has no layout, so contrast cannot be computed.
      "color-contrast": { enabled: false },
      // Components are tested outside of a page with landmarks.
      region: { enabled: false },
    },
  });
  const summary = result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.html).join(", ")}`);
  expect(summary).toEqual([]);
}
