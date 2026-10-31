// The in-app help is the user guide from docs/, bundled at build time, so
// the repository and the app show the same text.

import en from "../../../../docs/user-guide.md?raw";
import ru from "../../../../docs/user-guide.ru.md?raw";

const DOCS_URL = "https://github.com/hightemp/freelance-crm/blob/main/docs/";

export interface GuideSection {
  id: string;
  title: string;
  /** Markdown. */
  body: string;
}

export interface Guide {
  title: string;
  /** Markdown before the first section. */
  intro: string;
  sections: GuideSection[];
}

/** Splits a guide into its `##` sections. */
export function parseGuide(source: string): Guide {
  const text = source
    // The file links to its translation; the app shows the right one itself.
    .replace(/^>.*\]\(user-guide(\.ru)?\.md\).*\n+/m, "")
    // Other documents are not in the app: open them in the repository.
    .replace(/\]\(([\w.-]+\.md)\)/g, (_, file: string) => `](${DOCS_URL}${file})`);
  const [head, ...parts] = text.split(/^## /m);
  const title = /^# (.+)$/m.exec(head)?.[1].trim() ?? "";
  return {
    title,
    intro: head.replace(/^# .+$/m, ""),
    sections: parts.map((part, i) => {
      const newline = part.indexOf("\n");
      return {
        id: `help-${i}`,
        title: part.slice(0, newline).trim(),
        body: part.slice(newline + 1),
      };
    }),
  };
}

export const GUIDES = { en: parseGuide(en), ru: parseGuide(ru) };

const fold = (s: string) => s.toLowerCase().replaceAll("ё", "е");

/** Whether the section's title or text contains the query. */
export function matches(section: GuideSection, query: string): boolean {
  const q = fold(query.trim());
  return q === "" || fold(`${section.title}\n${section.body}`).includes(q);
}
