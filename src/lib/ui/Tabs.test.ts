import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import { textWithArg } from "../../test/snippets.svelte";
import Tabs from "./Tabs.svelte";

const tabs = [
  { id: "overview", label: "Overview" },
  { id: "contacts", label: "Contacts" },
  { id: "files", label: "Files", disabled: true },
  { id: "notes", label: "Notes" },
];

function setup(initial = "overview") {
  const { state, props } = bind(
    {
      label: "Client sections",
      tabs,
      panel: textWithArg("panel"),
    },
    "value",
    initial,
  );
  render(Tabs, props);
  return state;
}

describe("Tabs", () => {
  it("renders tablist, selected tab and its panel", () => {
    setup();
    expect(screen.getByRole("tablist", { name: "Client sections" })).toBeInTheDocument();
    const tab = screen.getByRole("tab", { name: "Overview" });
    expect(tab).toHaveAttribute("aria-selected", "true");
    const panel = screen.getByRole("tabpanel");
    expect(panel).toHaveTextContent("panel:overview");
    expect(tab).toHaveAttribute("aria-controls", panel.id);
    expect(panel).toHaveAccessibleName("Overview");
  });

  it("switches on click", async () => {
    const state = setup();
    await userEvent.click(screen.getByRole("tab", { name: "Contacts" }));
    expect(state.value).toBe("contacts");
    expect(screen.getByRole("tabpanel")).toHaveTextContent("panel:contacts");
  });

  it("arrows, Home and End activate tabs and skip disabled", async () => {
    const state = setup();
    screen.getByRole("tab", { name: "Overview" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(state.value).toBe("contacts");
    await userEvent.keyboard("{ArrowRight}");
    expect(state.value).toBe("notes");
    expect(screen.getByRole("tab", { name: "Notes" })).toHaveFocus();
    await userEvent.keyboard("{ArrowRight}");
    expect(state.value).toBe("overview");
    await userEvent.keyboard("{End}");
    expect(state.value).toBe("notes");
    await userEvent.keyboard("{Home}");
    expect(state.value).toBe("overview");
  });

  it("has no a11y violations", async () => {
    setup();
    await expectNoA11yViolations(document.body);
  });
});

describe("Tabs fill", () => {
  it("lets the panel take the remaining height", () => {
    render(Tabs, {
      label: "Sections",
      tabs,
      value: "overview",
      fill: true,
      panel: textWithArg("Panel"),
    });
    expect(screen.getByRole("tabpanel")).toHaveClass("flex-1", "min-h-0");
  });
});
