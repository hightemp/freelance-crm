import { describe, expect, it } from "vitest";
import type { SearchHit } from "$lib/api/bindings";
import { hitHref } from "./target";

const hit = (patch: Partial<SearchHit>): SearchHit =>
  ({ kind: "client", id: "x1", owner: null, title: "", snippet: null, ...patch }) as SearchHit;
const client = { type: "client", id: "c1" } as const;

describe("hitHref", () => {
  it("opens records in their cards", () => {
    expect(hitHref(hit({ kind: "client", id: "c1" }))).toBe("/clients/c1");
    expect(hitHref(hit({ kind: "project", id: "p1" }))).toBe("/projects/p1");
    expect(hitHref(hit({ kind: "task", id: "t1" }))).toBe("/tasks/t1");
    expect(hitHref(hit({ kind: "secret", id: "s1" }))).toBe("/vault/s1");
    expect(hitHref(hit({ kind: "invoice", id: "i1" }))).toBe("/finance?invoice=i1");
    expect(hitHref(hit({ kind: "event", id: "e1" }))).toBe("/calendar");
  });

  it("opens the client's tab for what belongs to a client", () => {
    expect(hitHref(hit({ kind: "contact", owner: client }))).toBe("/clients/c1?tab=contacts");
    expect(hitHref(hit({ kind: "interaction", owner: client }))).toBe("/clients/c1?tab=history");
    expect(hitHref(hit({ kind: "note", owner: client }))).toBe("/clients/c1?tab=notes");
    expect(hitHref(hit({ kind: "field", owner: client }))).toBe("/clients/c1?tab=fields");
    expect(hitHref(hit({ kind: "file", owner: client }))).toBe("/clients/c1?tab=files");
  });

  it("opens the card of other owners", () => {
    expect(hitHref(hit({ kind: "note", owner: { type: "project", id: "p1" } }))).toBe(
      "/projects/p1",
    );
    expect(hitHref(hit({ kind: "file", owner: { type: "task", id: "t1" } }))).toBe("/tasks/t1");
    expect(hitHref(hit({ kind: "note", owner: { type: "invoice", id: "i1" } }))).toBe(
      "/finance?invoice=i1",
    );
    expect(hitHref(hit({ kind: "file", owner: { type: "expense", id: "x1" } }))).toBe("/finance");
  });
});
