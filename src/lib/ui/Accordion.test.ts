import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { textWithArg } from "../../test/snippets.svelte";
import Accordion from "./Accordion.svelte";

const items = [
  { id: "bank", title: "Bank details" },
  { id: "address", title: "Address" },
];

describe("Accordion", () => {
  it("starts collapsed and expands a section on click", async () => {
    render(Accordion, { items, content: textWithArg("content") });
    const header = screen.getByRole("button", { name: "Bank details" });
    expect(header).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("content:bank")).not.toBeInTheDocument();
    await userEvent.click(header);
    expect(header).toHaveAttribute("aria-expanded", "true");
    const region = screen.getByRole("region", { name: "Bank details" });
    expect(region).toHaveTextContent("content:bank");
    expect(header).toHaveAttribute("aria-controls", region.id);
  });

  it("single mode collapses the other section", async () => {
    render(Accordion, { items, content: textWithArg("c") });
    await userEvent.click(screen.getByRole("button", { name: "Bank details" }));
    await userEvent.click(screen.getByRole("button", { name: "Address" }));
    expect(screen.getByRole("button", { name: "Bank details" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("multiple mode keeps sections open", async () => {
    render(Accordion, { items, content: textWithArg("c"), multiple: true });
    await userEvent.click(screen.getByRole("button", { name: "Bank details" }));
    await userEvent.click(screen.getByRole("button", { name: "Address" }));
    expect(screen.getAllByRole("region")).toHaveLength(2);
  });

  it("toggles with the keyboard", async () => {
    render(Accordion, { items, content: textWithArg("c") });
    screen.getByRole("button", { name: "Address" }).focus();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("region", { name: "Address" })).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(Accordion, { items, content: textWithArg("c"), open: ["bank"] });
    await expectNoA11yViolations(container);
  });
});
