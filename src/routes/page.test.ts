import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Page from "./+page.svelte";

describe("home page", () => {
  it("renders the app heading", () => {
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("freelance-crm");
  });
});
