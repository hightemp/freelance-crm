import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Interaction_Serialize as Interaction } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { typeInEditor } from "../../../test/editor";
import InteractionFeed from "./InteractionFeed.svelte";

const at = (s: string) => Date.parse(s);

function item(
  id: string,
  kind: Interaction["kind"],
  when: string,
  summary: string,
  patch: Partial<Interaction> = {},
): Interaction {
  return {
    id,
    clientId: "c1",
    contactId: null,
    contactName: null,
    projectId: null,
    projectName: null,
    kind,
    occurredAt: at(when),
    summaryMd: summary,
    followUps: [],
    createdAt: 1,
    updatedAt: 1,
    ...patch,
  };
}

const PAGE_1 = [
  item("i3", "call", "2026-03-12T10:00:00Z", "Обсудили **сроки**", {
    contactId: "k1",
    contactName: "Петрова Анна",
  }),
  item("i2", "email", "2026-03-11T09:00:00Z", "Прислали ТЗ", {
    followUps: [{ kind: "task", id: "t1", title: "Прочитать ТЗ" }],
  }),
];
const PAGE_2 = [item("i1", "meeting", "2026-03-01T15:00:00Z", "Знакомство")];

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let observed: ((entries: { isIntersecting: boolean }[]) => void) | null = null;

class FakeObserver {
  constructor(callback: (entries: { isIntersecting: boolean }[]) => void) {
    observed = callback;
  }
  observe() {}
  disconnect() {}
}

beforeEach(() => {
  calls = [];
  observed = null;
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "interaction_feed": {
        if (a.kind === "meeting") return { items: PAGE_2, next: null };
        return a.after
          ? { items: PAGE_2, next: null }
          : { items: PAGE_1, next: { occurredAt: PAGE_1[1].occurredAt, id: "i2" } };
      }
      case "create_interaction":
        return item("i9", "call", "2026-03-18T09:00:00Z", "x");
      case "follow_up_task":
        return { kind: "task", id: "t2", title: a.title };
      case "list_contacts":
        return [{ id: "k1", displayName: "Петрова Анна" }];
      case "list_projects":
        return [];
      case "delete_interaction":
        return null;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
  vi.unstubAllGlobals();
});

const feedCalls = () => calls.filter((c) => c.cmd === "interaction_feed").map((c) => c.args);

function show() {
  render(DialogHost);
  return render(InteractionFeed, { clientId: "c1", now: () => at("2026-03-18T09:00:00Z") });
}

describe("InteractionFeed", () => {
  it("shows the history newest first", async () => {
    show();
    const feed = await screen.findByRole("feed", { name: "History" });
    const articles = within(feed).getAllByRole("article");
    expect(articles).toHaveLength(2);
    expect(articles[0]).toHaveTextContent("Call");
    expect(articles[0]).toHaveTextContent("Петрова Анна");
    expect(within(articles[0]).getByText("сроки").tagName).toBe("STRONG");
    expect(within(articles[1]).getByRole("link", { name: "Прочитать ТЗ" })).toHaveAttribute(
      "href",
      "/tasks/t1",
    );
    expect(feedCalls()[0]).toEqual({ clientId: "c1", kind: null, after: null, limit: 20 });
  });

  it("loads the next page when the end scrolls into view", async () => {
    show();
    await screen.findByText("Прислали ТЗ");
    observed!([{ isIntersecting: true }]);
    expect(await screen.findByText("Знакомство")).toBeInTheDocument();
    expect(feedCalls()[1].after).toEqual({ occurredAt: PAGE_1[1].occurredAt, id: "i2" });
    expect(screen.getAllByRole("article")).toHaveLength(3);
    // Nothing more to load.
    observed!([{ isIntersecting: true }]);
    await new Promise((r) => setTimeout(r, 10));
    expect(feedCalls()).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Load more" })).not.toBeInTheDocument();
  });

  it("filters by type", async () => {
    show();
    await screen.findByText("Прислали ТЗ");
    await userEvent.click(screen.getByRole("combobox", { name: "Type" }));
    await userEvent.click(screen.getByRole("option", { name: "Meeting" }));
    expect(await screen.findByText("Знакомство")).toBeInTheDocument();
    expect(screen.queryByText("Прислали ТЗ")).not.toBeInTheDocument();
    expect(feedCalls().at(-1)).toMatchObject({ kind: "meeting", after: null });
  });

  it("adds a record", async () => {
    show();
    await screen.findByText("Прислали ТЗ");
    await userEvent.click(screen.getByRole("button", { name: "Add record" }));
    const form = screen.getByRole("form", { name: "New record" });
    await userEvent.click(within(form).getByRole("combobox", { name: "Contact" }));
    await userEvent.click(await screen.findByRole("option", { name: "Петрова Анна" }));
    typeInEditor(within(form).getByRole("textbox", { name: "Summary" }), "Перезвонить в пятницу");
    await userEvent.click(within(form).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "create_interaction")).toBe(true));
    expect(calls.find((c) => c.cmd === "create_interaction")!.args.input).toEqual({
      clientId: "c1",
      contactId: "k1",
      projectId: null,
      kind: "call",
      occurredAt: at("2026-03-18T09:00:00Z"),
      summaryMd: "Перезвонить в пятницу",
    });
    // The feed starts over to show it.
    await waitFor(() => expect(feedCalls().at(-1)).toMatchObject({ after: null }));
  });

  it("creates a follow-up task", async () => {
    show();
    const first = (await screen.findAllByRole("article"))[0];
    await userEvent.click(within(first).getByRole("button", { name: "Create task" }));
    const dialog = await screen.findByRole("dialog", { name: "Create task" });
    await userEvent.type(within(dialog).getByRole("textbox"), "Отправить договор");
    await userEvent.click(within(dialog).getByRole("button", { name: "Create" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "follow_up_task")?.args).toEqual({
        id: "i3",
        title: "Отправить договор",
        dueDate: null,
      }),
    );
  });

  it("has no a11y violations", async () => {
    const { container } = show();
    await screen.findByText("Прислали ТЗ");
    await expectNoA11yViolations(container);
  });
});
