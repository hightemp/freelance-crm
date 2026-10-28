import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { setAppLocale } from "../i18n/locale.svelte";
import AppSidebar from "./AppSidebar.svelte";

beforeEach(() =>
  mockIPC((cmd, args) =>
    cmd === "save_settings" ? (args as { settings: unknown }).settings : undefined,
  ),
);
afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const SECTIONS = [
  ["Home", "/"],
  ["Clients", "/clients"],
  ["Projects", "/projects"],
  ["Tasks", "/tasks"],
  ["Calendar", "/calendar"],
  ["Time", "/time"],
  ["Finance", "/finance"],
  ["Vault", "/vault"],
  ["Settings", "/settings"],
];

describe("AppSidebar", () => {
  it("links to every section", () => {
    render(AppSidebar, { currentPath: "/" });
    const nav = screen.getByRole("navigation", { name: "Main" });
    expect(nav).toBeInTheDocument();
    for (const [name, href] of SECTIONS) {
      expect(screen.getByRole("link", { name })).toHaveAttribute("href", href);
    }
  });

  it("marks the current section, including nested routes", () => {
    render(AppSidebar, { currentPath: "/clients/42" });
    expect(screen.getByRole("link", { name: "Clients" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Home" })).not.toHaveAttribute("aria-current");
  });

  it("marks Home only on the root path", () => {
    render(AppSidebar, { currentPath: "/" });
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("aria-current", "page");
  });

  it("is localized", () => {
    setAppLocale("ru");
    render(AppSidebar, { currentPath: "/" });
    expect(screen.getByRole("link", { name: "Клиенты" })).toBeInTheDocument();
  });

  it("every link is reachable with Tab in order", async () => {
    render(AppSidebar, { currentPath: "/" });
    for (const [name] of SECTIONS) {
      await userEvent.tab();
      expect(screen.getByRole("link", { name })).toHaveFocus();
    }
  });

  it("switches theme from the theme menu", async () => {
    render(AppSidebar, { currentPath: "/" });
    await userEvent.click(screen.getByRole("button", { name: "Theme" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Dark" }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    await userEvent.click(screen.getByRole("button", { name: "Theme" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Light" }));
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("has no a11y violations", async () => {
    const { container } = render(AppSidebar, { currentPath: "/tasks" });
    await expectNoA11yViolations(container);
  });
});
