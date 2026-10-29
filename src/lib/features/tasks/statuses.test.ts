import { afterEach, describe, expect, it } from "vitest";
import type { TaskStatus } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { statusName } from "./statuses";

const column = (patch: Partial<TaskStatus>): TaskStatus => ({
  id: "s1",
  name: null,
  nameKey: "todo",
  category: "todo",
  color: "#000000",
  position: 0,
  wipLimit: null,
  ...patch,
});

afterEach(() => setAppLocale("en"));

describe("statusName", () => {
  it("localizes default columns until they are renamed", () => {
    expect(statusName(column({ nameKey: "review" }))).toBe("Review");
    setAppLocale("ru");
    expect(statusName(column({ nameKey: "review" }))).toBe("На проверке");
    expect(statusName(column({ name: "Бэклог" }))).toBe("Бэклог");
  });

  it("falls back to the key for unknown defaults", () => {
    expect(statusName(column({ nameKey: "archive" }))).toBe("archive");
  });
});
