import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Tag } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { bind } from "../../../test/bind.svelte";
import TagEditor from "./TagEditor.svelte";

let tags: Tag[] = [];
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  tags = [
    { id: "t1", name: "VIP", color: "#ef4444", createdAt: 0 },
    { id: "t2", name: "Срочно", color: "#3b82f6", createdAt: 0 },
  ];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    if (cmd === "list_tags") return tags;
    if (cmd === "create_tag") {
      const tag = { id: "t-new", ...(a.input as object), createdAt: 0 } as Tag;
      tags = [...tags, tag];
      return tag;
    }
    if (cmd === "set_tags") return tags.filter((t) => (a.tagIds as string[]).includes(t.id));
  });
});

afterEach(() => clearMocks());

function setup(initial: string[] = ["t1"]) {
  const { state, props } = bind(
    { owner: { type: "task", id: "x1" } as const, label: "Tags" },
    "tagIds",
    initial,
  );
  render(TagEditor, props);
  return { state, input: () => screen.getByRole("combobox", { name: "Tags" }) };
}

describe("TagEditor", () => {
  it("shows the attached tags by name", async () => {
    setup();
    expect(await screen.findByText("VIP")).toBeInTheDocument();
  });

  it("attaches existing tags case-insensitively and creates new ones", async () => {
    const { state, input } = setup();
    await screen.findByText("VIP");
    await userEvent.type(input(), "срочно{Enter}");
    await waitFor(() => expect(state.value).toEqual(["t1", "t2"]));
    expect(calls.some((c) => c.cmd === "create_tag")).toBe(false);
    expect(calls.findLast((c) => c.cmd === "set_tags")!.args).toEqual({
      target: { type: "task", id: "x1" },
      tagIds: ["t1", "t2"],
    });

    await userEvent.type(input(), "Новый{Enter}");
    await waitFor(() => expect(state.value).toEqual(["t1", "t2", "t-new"]));
    expect((calls.find((c) => c.cmd === "create_tag")!.args.input as Tag).name).toBe("Новый");
  });

  it("detaches tags", async () => {
    const { state } = setup();
    await userEvent.click(await screen.findByRole("button", { name: "Remove VIP" }));
    await waitFor(() => expect(state.value).toEqual([]));
  });

  it("has no a11y violations", async () => {
    const { container } = render(TagEditor, {
      owner: { type: "client", id: "c1" },
      label: "Tags",
      tagIds: [],
    });
    await expectNoA11yViolations(container);
  });
});
