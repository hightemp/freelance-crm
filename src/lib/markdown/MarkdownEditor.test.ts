import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import { editorText, typeInEditor } from "../../test/editor";
import MarkdownEditor from "./MarkdownEditor.svelte";

function setup(initial = "") {
  const { state, props } = bind({ label: "Body" }, "value", initial);
  render(MarkdownEditor, props);
  return { state, box: () => screen.getByRole("textbox", { name: "Body" }) };
}

describe("MarkdownEditor", () => {
  it("is a labelled multiline textbox with the initial text", () => {
    const { box } = setup("# Hi");
    expect(box()).toHaveAttribute("aria-multiline", "true");
    expect(editorText(box())).toBe("# Hi");
  });

  it("updates the bound value as the user types", () => {
    const { state, box } = setup("a");
    typeInEditor(box(), "bc");
    expect(state.value).toBe("abc");
  });

  it("follows external value changes", async () => {
    const { state, box } = setup("draft");
    state.value = "";
    await waitFor(() => expect(editorText(box())).toBe(""));
  });

  it("previews rendered markdown and keeps the text when switching back", async () => {
    const { box } = setup("**bold**");
    await userEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(screen.getByRole("button", { name: "Preview" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("bold").tagName).toBe("STRONG");
    expect(screen.queryByRole("textbox", { name: "Body" })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Write" }));
    expect(editorText(box())).toBe("**bold**");
  });

  it("says when there is nothing to preview", async () => {
    setup("");
    await userEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(screen.getByText("Nothing to preview")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { box } = setup("text");
    await expectNoA11yViolations(box().closest("div[data-md-editor]")!);
  });
});
