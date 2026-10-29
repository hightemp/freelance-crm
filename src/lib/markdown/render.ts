// Markdown → sanitized HTML for notes and descriptions. Everything users
// write is untrusted: HTML passes through DOMPurify, links never navigate
// the app window and remote images are dropped so notes cannot phone home.

import DOMPurify from "dompurify";
import { Marked } from "marked";
import { m } from "$lib/paraglide/messages";

const marked = new Marked({
  gfm: true,
  breaks: true,
  renderer: {
    // A styled mark instead of a native checkbox (FR-UI-8).
    checkbox({ checked }) {
      const label = checked ? m.md_task_done() : m.md_task_todo();
      return `<span class="md-task" role="img" aria-label="${label}" data-checked="${checked}"></span> `;
    },
  },
});

const purify = DOMPurify();

purify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "A") {
    node.removeAttribute("target");
    node.setAttribute("rel", "noopener noreferrer");
  }
  if (node.tagName === "IMG" && !node.getAttribute("src")?.startsWith("data:")) {
    node.remove();
  }
});

export function renderMarkdown(source: string): string {
  if (source.trim() === "") return "";
  const html = marked.parse(source, { async: false });
  return purify.sanitize(html, { FORBID_TAGS: ["form", "input", "button", "textarea", "select"] });
}
