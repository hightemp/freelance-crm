import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import ExportDialog from "./ExportDialog.svelte";
import ImportDialog from "./ImportDialog.svelte";

const icsEvent = (uid: string, title: string, patch: Record<string, unknown> = {}) => ({
  uid,
  title,
  description: "",
  location: null,
  allDay: false,
  start: "2026-03-02T10:00",
  end: "2026-03-02T10:30",
  tzid: "Europe/Moscow",
  rrule: null,
  exdates: [],
  recurrenceId: null,
  reminders: [],
  ...patch,
});

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "plugin:dialog|open":
        return "/home/me/work.ics";
      case "plugin:dialog|save":
        return "/home/me/export.ics";
      case "preview_ics":
        return {
          events: [
            icsEvent("standup", "Планёрка", { rrule: "FREQ=WEEKLY;BYDAY=MO" }),
            icsEvent("standup", "Планёрка (перенос)", { recurrenceId: "2026-03-09T10:00" }),
            icsEvent("holidays", "Майские", {
              allDay: true,
              start: "2026-05-01",
              end: "2026-05-03",
            }),
          ],
          issues: [
            {
              uid: "broken",
              line: 12,
              error: { code: "ics.no_start", params: {} },
              warning: false,
            },
          ],
        };
      case "import_ics":
        return { created: 1, updated: 2, issues: [] };
      case "export_ics":
        return null;
      case "list_clients":
        return [{ id: "c1", displayName: "Ромашка" }];
      case "list_projects":
        return [{ id: "p1", name: "Сайт", clientId: "c1", clientName: "Ромашка" }];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const named = (cmd: string) => calls.filter((c) => c.cmd === cmd).map((c) => c.args);

describe("ImportDialog", () => {
  it("previews a file, imports the chosen events and reports", async () => {
    const onclose = vi.fn();
    render(ToastHost);
    render(ImportDialog, { onclose });
    const dialog = await screen.findByRole("dialog", { name: "Import a calendar" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Choose an .ics file" }));
    // One row per event; the edited occurrence goes with its series.
    const standup = await within(dialog).findByRole("checkbox", { name: /Планёрка/ });
    expect(within(dialog).getAllByRole("checkbox", { name: /Планёрка|Майские/ })).toHaveLength(2);
    expect(standup).toBeChecked();
    expect(within(dialog).getByText("Repeats")).toBeInTheDocument();
    expect(within(dialog).getByText(/Line 12/)).toHaveTextContent("The event has no start");

    await userEvent.click(within(dialog).getByRole("checkbox", { name: /Майские/ }));
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Link to a client" }));
    await userEvent.click(screen.getByRole("option", { name: "Ромашка" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Import (1)" }));
    await waitFor(() => expect(onclose).toHaveBeenCalled());
    expect(named("import_ics")).toEqual([
      {
        path: "/home/me/work.ics",
        uids: ["standup"],
        link: { clientId: "c1", projectId: null },
      },
    ]);
    expect(await screen.findByText("Imported: 1 new, 2 updated")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    render(ImportDialog, { onclose: vi.fn() });
    const dialog = await screen.findByRole("dialog", { name: "Import a calendar" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Choose an .ics file" }));
    await within(dialog).findByRole("checkbox", { name: /Планёрка/ });
    await expectNoA11yViolations(dialog);
  });
});

describe("ExportDialog", () => {
  it("exports everything by default", async () => {
    const onclose = vi.fn();
    render(ExportDialog, { onclose });
    const dialog = await screen.findByRole("dialog", { name: "Export a calendar" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Save .ics" }));
    await waitFor(() => expect(onclose).toHaveBeenCalled());
    expect(named("export_ics")).toEqual([
      {
        path: "/home/me/export.ics",
        filter: { clientId: null, projectId: null, from: null, to: null },
      },
    ]);
  });

  it("exports a client's events", async () => {
    render(ExportDialog, { onclose: vi.fn() });
    const dialog = await screen.findByRole("dialog", { name: "Export a calendar" });
    await userEvent.click(within(dialog).getByRole("radio", { name: "A client's events" }));
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Client" }));
    await userEvent.click(await screen.findByRole("option", { name: "Ромашка" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Save .ics" }));
    await waitFor(() => expect(named("export_ics")).toHaveLength(1));
    expect(named("export_ics")[0].filter).toEqual({
      clientId: "c1",
      projectId: null,
      from: null,
      to: null,
    });
  });

  it("has no a11y violations", async () => {
    render(ExportDialog, { onclose: vi.fn() });
    await expectNoA11yViolations(await screen.findByRole("dialog", { name: "Export a calendar" }));
  });
});
