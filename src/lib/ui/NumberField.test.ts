import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import { setAppLocale } from "../i18n/locale.svelte";
import MoneyField from "./MoneyField.svelte";
import NumberField from "./NumberField.svelte";

afterEach(() => setAppLocale("en"));

describe("NumberField", () => {
  it("is a text input with decimal keyboard, no native spinners", () => {
    const { props } = bind({ label: "Hours" }, "value", null);
    render(NumberField, props);
    const input = screen.getByRole("textbox", { name: "Hours" });
    expect(input).toHaveAttribute("type", "text");
    expect(input).toHaveAttribute("inputmode", "decimal");
  });

  it("parses localized input on blur", async () => {
    setAppLocale("ru");
    const { state, props } = bind({ label: "Hours" }, "value", null);
    render(NumberField, props);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "1 234,5");
    await userEvent.tab();
    expect(state.value).toBe(1234.5);
    expect((input as HTMLInputElement).value.replace(/\s/g, " ")).toBe("1 234,5");
  });

  it("shows an error for invalid input and keeps the previous value", async () => {
    const { state, props } = bind({ label: "Hours" }, "value", 3);
    render(NumberField, props);
    const input = screen.getByRole("textbox");
    await userEvent.clear(input);
    await userEvent.type(input, "abc");
    await userEvent.tab();
    expect(state.value).toBe(3);
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAccessibleDescription("Invalid format");
  });

  it("clamps to min/max and steps with arrow keys", async () => {
    const { state, props } = bind({ label: "Qty", min: 0, max: 10, step: 2 }, "value", 9);
    render(NumberField, props);
    const input = screen.getByRole("textbox");
    input.focus();
    await userEvent.keyboard("{ArrowUp}");
    expect(state.value).toBe(10);
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(state.value).toBe(6);
    await userEvent.clear(input);
    await userEvent.type(input, "-5");
    await userEvent.tab();
    expect(state.value).toBe(0);
  });

  it("empty input means null", async () => {
    const { state, props } = bind({ label: "Hours" }, "value", 5);
    render(NumberField, props);
    await userEvent.clear(screen.getByRole("textbox"));
    await userEvent.tab();
    expect(state.value).toBeNull();
  });

  it("has no a11y violations", async () => {
    const { props } = bind({ label: "Hours" }, "value", 1);
    const { container } = render(NumberField, props);
    await expectNoA11yViolations(container);
  });
});

describe("MoneyField", () => {
  it("stores minor units and shows the currency", async () => {
    setAppLocale("ru");
    const { state, props } = bind({ label: "Rate", currency: "RUB" }, "value", null);
    render(MoneyField, props);
    expect(screen.getByText("RUB")).toBeInTheDocument();
    const input = screen.getByRole("textbox", { name: "Rate" });
    await userEvent.type(input, "2 500,5");
    await userEvent.tab();
    expect(state.value).toBe(250050);
    expect((input as HTMLInputElement).value.replace(/\s/g, " ")).toBe("2 500,50");
  });

  it("respects currency precision", async () => {
    const { state, props } = bind({ label: "Price", currency: "JPY" }, "value", null);
    render(MoneyField, props);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "1.5");
    await userEvent.tab();
    expect(state.value).toBeNull();
    expect(input).toHaveAttribute("aria-invalid", "true");
    await userEvent.clear(input);
    await userEvent.type(input, "1500");
    await userEvent.tab();
    expect(state.value).toBe(1500);
  });

  it("shows an existing value formatted", () => {
    const { props } = bind({ label: "Rate", currency: "USD" }, "value", 123456);
    render(MoneyField, props);
    expect(screen.getByRole("textbox")).toHaveValue("1,234.56");
  });
});
