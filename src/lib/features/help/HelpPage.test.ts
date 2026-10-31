import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import HelpPage from "./HelpPage.svelte";

afterEach(() => {
  cleanup();
  setAppLocale("en");
});

describe("HelpPage", () => {
  it("shows the user guide with its contents", async () => {
    const { container } = render(HelpPage);
    expect(screen.getByRole("heading", { level: 1, name: "User guide" })).toBeInTheDocument();
    const contents = screen.getByRole("navigation", { name: "Contents" });
    expect(within(contents).getByRole("button", { name: "Backups" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Backups" })).toBeInTheDocument();
    await expectNoA11yViolations(container);
  });

  it("scrolls to a section from the contents", async () => {
    render(HelpPage);
    const section = screen.getByRole("region", { name: "Vault" });
    const scroll = vi.fn();
    section.scrollIntoView = scroll;
    const contents = screen.getByRole("navigation", { name: "Contents" });
    await userEvent.click(within(contents).getByRole("button", { name: "Vault" }));
    expect(scroll).toHaveBeenCalled();
  });

  it("filters the sections", async () => {
    render(HelpPage);
    await userEvent.type(
      screen.getByRole("searchbox", { name: "Search the help" }),
      "master password",
    );
    expect(screen.getByRole("region", { name: "Vault" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Calendar" })).toBeNull();

    await userEvent.clear(screen.getByRole("searchbox", { name: "Search the help" }));
    await userEvent.type(screen.getByRole("searchbox", { name: "Search the help" }), "zzzz");
    expect(screen.getByText("Nothing found")).toBeInTheDocument();
  });

  it("follows the interface language", () => {
    setAppLocale("ru");
    render(HelpPage);
    expect(
      screen.getByRole("heading", { level: 1, name: "Руководство пользователя" }),
    ).toBeInTheDocument();
  });
});
