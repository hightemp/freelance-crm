import { afterEach, describe, expect, it } from "vitest";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { renderMarkdown } from "./render";

function dom(md: string): HTMLElement {
  const el = document.createElement("div");
  el.innerHTML = renderMarkdown(md);
  return el;
}

afterEach(() => setAppLocale("en"));

describe("renderMarkdown", () => {
  it("renders basic markdown", () => {
    const el = dom("# Title\n\nSome **bold** and *italic* text\nwith a line break");
    expect(el.querySelector("h1")?.textContent).toBe("Title");
    expect(el.querySelector("strong")?.textContent).toBe("bold");
    expect(el.querySelector("em")?.textContent).toBe("italic");
    expect(el.querySelector("br")).not.toBeNull();
  });

  it.each([
    ["<script>alert(1)</script>", "script"],
    ['<img src="x" onerror="alert(1)">', "[onerror]"],
    ['<a href="javascript:alert(1)">x</a>', 'a[href^="javascript"]'],
    ["[x](javascript:alert(1))", 'a[href^="javascript"]'],
    ['<iframe src="https://example.com"></iframe>', "iframe"],
    ['<div onclick="alert(1)">x</div>', "[onclick]"],
    ["<svg><script>alert(1)</script></svg>", "script"],
    ['<form action="https://evil"><input></form>', "form, input"],
  ])("strips dangerous markup: %s", (md, selector) => {
    expect(dom(md).querySelector(selector)).toBeNull();
  });

  it("renders GFM tables and strikethrough", () => {
    const el = dom("| Name | Rate |\n| --- | ---: |\n| Ромашка | 2500 |\n\n~~old~~");
    expect([...el.querySelectorAll("th")].map((th) => th.textContent)).toEqual(["Name", "Rate"]);
    expect(el.querySelector("td")?.textContent).toBe("Ромашка");
    expect(el.querySelector("del")?.textContent).toBe("old");
  });

  it("renders task lists with custom, labelled check marks", () => {
    const el = dom("- [x] Done\n- [ ] Todo");
    expect(el.querySelector("input")).toBeNull();
    const marks = [...el.querySelectorAll('[role="img"]')];
    expect(marks.map((m) => m.getAttribute("aria-label"))).toEqual(["Done", "Not done"]);
    expect(marks.map((m) => m.getAttribute("data-checked"))).toEqual(["true", "false"]);
  });

  it("labels check marks in the current language", () => {
    setAppLocale("ru");
    expect(dom("- [x] a").querySelector('[role="img"]')?.getAttribute("aria-label")).toBe(
      "Выполнено",
    );
  });

  it("makes links safe to open externally", () => {
    const a = dom(
      '[site](https://example.com) <a href="https://x.y" target="_self">x</a>',
    ).querySelectorAll("a");
    expect(a).toHaveLength(2);
    for (const link of a) {
      expect(link.getAttribute("rel")).toBe("noopener noreferrer");
      expect(link.hasAttribute("target")).toBe(false);
    }
  });

  it("drops remote images so notes never phone home", () => {
    const el = dom("![pixel](https://tracker.example/p.gif) ![inline](data:image/png;base64,AAAA)");
    const images = [...el.querySelectorAll("img")];
    expect(images.map((i) => i.getAttribute("src"))).toEqual(["data:image/png;base64,AAAA"]);
  });

  it("returns an empty string for empty input", () => {
    expect(renderMarkdown("")).toBe("");
  });
});
