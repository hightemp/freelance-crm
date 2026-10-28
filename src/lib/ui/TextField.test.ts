import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import TextArea from "./TextArea.svelte";
import TextField from "./TextField.svelte";

describe("TextField", () => {
  it("is labelled and reports typed value", async () => {
    let value = "";
    render(TextField, {
      label: "Email",
      get value() {
        return value;
      },
      set value(v: string) {
        value = v;
      },
    });
    await userEvent.type(screen.getByLabelText("Email"), "a@b.c");
    expect(value).toBe("a@b.c");
  });

  it("links hint and error via aria-describedby and marks invalid", () => {
    render(TextField, { label: "Name", hint: "Full name", error: "Required" });
    const input = screen.getByLabelText("Name");
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAccessibleDescription("Full name Required");
  });

  it("is valid without error", () => {
    render(TextField, { label: "Name" });
    expect(screen.getByLabelText("Name")).toHaveAttribute("aria-invalid", "false");
  });

  it("supports only text-like input types", () => {
    render(TextField, { label: "Secret", type: "password" });
    expect(screen.getByLabelText("Secret")).toHaveAttribute("type", "password");
  });

  it("marks required fields", () => {
    render(TextField, { label: "Title", required: true });
    expect(screen.getByLabelText(/Title/)).toHaveAttribute("aria-required", "true");
  });

  it("has no a11y violations", async () => {
    const { container } = render(TextField, { label: "Email", error: "Bad" });
    await expectNoA11yViolations(container);
  });
});

describe("TextArea", () => {
  it("is labelled, multi-line and describes errors", async () => {
    render(TextArea, { label: "Notes", error: "Too long" });
    const area = screen.getByLabelText("Notes");
    expect(area.tagName).toBe("TEXTAREA");
    expect(area).toHaveAccessibleDescription("Too long");
    await userEvent.type(area, "a{Enter}b");
    expect(area).toHaveValue("a\nb");
  });

  it("has no a11y violations", async () => {
    const { container } = render(TextArea, { label: "Notes" });
    await expectNoA11yViolations(container);
  });
});
