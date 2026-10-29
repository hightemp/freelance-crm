import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import VirtualListProbe from "../../test/VirtualListProbe.svelte";

const items = () => screen.getAllByRole("listitem");

describe("VirtualList", () => {
  it("renders only the visible rows of a long list", () => {
    render(VirtualListProbe, { count: 10_000 });
    // 320 / 32 = 10 visible rows plus 3 overscan below.
    expect(items()).toHaveLength(13);
    expect(items()[0]).toHaveTextContent("Row 0");
    expect(items()[0]).toHaveAttribute("aria-posinset", "1");
    expect(items()[0]).toHaveAttribute("aria-setsize", "10000");
  });

  it("follows scrolling", async () => {
    render(VirtualListProbe, { count: 10_000 });
    const list = screen.getByRole("list", { name: "Rows" });
    list.scrollTop = 32 * 5_000;
    await fireEvent.scroll(list);
    expect(items()).toHaveLength(16);
    expect(items()[0]).toHaveTextContent("Row 4997");
    expect(items()[3]).toHaveAttribute("aria-posinset", "5001");
  });

  it("renders short lists completely", () => {
    render(VirtualListProbe, { count: 4 });
    expect(items()).toHaveLength(4);
  });

  it("has no a11y violations", async () => {
    const { container } = render(VirtualListProbe, { count: 50 });
    await expectNoA11yViolations(container);
  });
});
