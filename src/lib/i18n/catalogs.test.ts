import { describe, expect, it } from "vitest";
import en from "../../../messages/en.json";
import ru from "../../../messages/ru.json";

const keys = (catalog: Record<string, string>) =>
  Object.keys(catalog)
    .filter((k) => k !== "$schema")
    .sort();

describe("message catalogs", () => {
  it("en and ru define the same keys", () => {
    expect(keys(ru)).toEqual(keys(en));
  });

  it.each([
    ["en", en],
    ["ru", ru],
  ])("%s has no empty messages", (_, catalog) => {
    for (const key of keys(catalog)) {
      expect(catalog[key as keyof typeof catalog], key).not.toBe("");
    }
  });

  it.each([
    ["en", en],
    ["ru", ru],
  ])("%s keys are snake_case", (_, catalog) => {
    for (const key of keys(catalog)) expect(key).toMatch(/^[a-z][a-z0-9_]*$/);
  });
});
