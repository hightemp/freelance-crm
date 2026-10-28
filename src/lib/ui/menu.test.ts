import { describe, expect, it } from "vitest";
import { typeaheadMatch, type MenuItem } from "./menu";

const items: MenuItem[] = [
  { id: "a", label: "Edit" },
  { id: "b", label: "Duplicate" },
  { id: "c", label: "Download", disabled: true },
  { id: "d", label: "Delete" },
];

describe("typeaheadMatch", () => {
  it.each([
    ["d", 0, 1],
    ["d", 1, 3],
    ["d", 3, 1],
    ["de", 1, 3],
    ["E", 3, 0],
    ["x", 0, -1],
  ])("%s from %d -> %d", (query, from, expected) => {
    expect(typeaheadMatch(items, query, from)).toBe(expected);
  });
});
