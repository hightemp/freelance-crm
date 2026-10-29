import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import Markdown from "./Markdown.svelte";

afterEach(() => clearMocks());

describe("Markdown", () => {
  it("renders sanitized markdown", () => {
    render(Markdown, { source: "## Plan\n\n- [x] call <script>alert(1)</script>" });
    expect(screen.getByRole("heading", { level: 2, name: "Plan" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Done" })).toBeInTheDocument();
    expect(document.querySelector("script")).toBeNull();
  });

  it("opens web links in the system browser, not in the app window", async () => {
    const opened: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "plugin:opener|open_url") opened.push((args as { url: string }).url);
    });
    render(Markdown, { source: "[site](https://example.com) [local](/clients)" });
    await userEvent.click(screen.getByRole("link", { name: "site" }));
    await userEvent.click(screen.getByRole("link", { name: "local" }));
    expect(opened).toEqual(["https://example.com"]);
  });

  it("has no a11y violations", async () => {
    const { container } = render(Markdown, { source: "| a | b |\n|---|---|\n| 1 | 2 |" });
    await expectNoA11yViolations(container);
  });
});
