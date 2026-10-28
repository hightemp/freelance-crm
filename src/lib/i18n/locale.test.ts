import { render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { detectLocale, i18n, setAppLocale } from "./locale.svelte";
import LocaleProbe from "../../test/LocaleProbe.svelte";

afterEach(() => setAppLocale("en"));

describe("detectLocale", () => {
  it.each([
    [["ru-RU", "en-US"], "ru"],
    [["en-GB"], "en"],
    [["de-DE", "ru"], "ru"],
    [["de-DE"], "en"],
    [[], "en"],
  ])("%j -> %s", (languages, expected) => {
    expect(detectLocale(languages)).toBe(expected);
  });
});

describe("setAppLocale", () => {
  it("updates state and html lang", () => {
    setAppLocale("ru");
    expect(i18n.locale).toBe("ru");
    expect(document.documentElement.lang).toBe("ru");
  });

  it("re-renders messages without reload", async () => {
    render(LocaleProbe);
    expect(screen.getByText("Home")).toBeInTheDocument();
    setAppLocale("ru");
    await tick();
    expect(screen.getByText("Главная")).toBeInTheDocument();
  });
});
