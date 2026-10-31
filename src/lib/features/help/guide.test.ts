import { describe, expect, it } from "vitest";
import { GUIDES, matches, parseGuide } from "./guide";

const SOURCE = `# User guide

> English version: [user-guide.md](user-guide.md)

Intro text.

## Getting started

Open **Settings**.

## Where the data is

See [security.md](security.md) and [the site](https://example.com).
`;

describe("parseGuide", () => {
  const guide = parseGuide(SOURCE);

  it("splits the title, the intro and the sections", () => {
    expect(guide.title).toBe("User guide");
    expect(guide.intro.trim()).toBe("Intro text.");
    expect(guide.sections.map((s) => s.title)).toEqual(["Getting started", "Where the data is"]);
    expect(guide.sections[0].body.trim()).toBe("Open **Settings**.");
    expect(new Set(guide.sections.map((s) => s.id)).size).toBe(2);
  });

  it("drops the link to the other language's file", () => {
    expect(guide.intro).not.toContain("user-guide.md");
  });

  it("points links to other documents at the repository", () => {
    expect(guide.sections[1].body).toContain(
      "[security.md](https://github.com/hightemp/freelance-crm/blob/main/docs/security.md)",
    );
    expect(guide.sections[1].body).toContain("[the site](https://example.com)");
  });
});

describe("the bundled guides", () => {
  it("have the same sections in both languages", () => {
    expect(GUIDES.en.sections.length).toBeGreaterThan(5);
    expect(GUIDES.ru.sections.length).toBe(GUIDES.en.sections.length);
  });
});

describe("matches", () => {
  const section = { id: "s", title: "Calendar", body: "Повторяющиеся события и ёлка" };

  it("finds words in the title and the text, ignoring case and ё", () => {
    expect(matches(section, "calendar")).toBe(true);
    expect(matches(section, "повторяющиеся")).toBe(true);
    expect(matches(section, "елка")).toBe(true);
    expect(matches(section, "invoice")).toBe(false);
    expect(matches(section, "  ")).toBe(true);
  });
});
