import { afterEach, describe, expect, it, vi } from "vitest";
import { gotoWithQuery } from "./navigate";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

afterEach(() => goto.mockReset());

describe("gotoWithQuery", () => {
  it("appends encoded query parameters", async () => {
    await gotoWithQuery("/tasks/new", { project: "p 1" });
    expect(goto).toHaveBeenCalledWith("/tasks/new?project=p+1");
  });

  it("navigates to the bare path without parameters", async () => {
    await gotoWithQuery("/tasks", {});
    expect(goto).toHaveBeenCalledWith("/tasks");
  });
});
