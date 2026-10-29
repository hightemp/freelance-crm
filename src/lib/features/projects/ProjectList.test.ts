import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type {
  ProjectFilter_Deserialize as ProjectFilter,
  Project_Serialize as Project,
} from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { client, project } from "./fixtures";
import ProjectList from "./ProjectList.svelte";

let projects: Project[] = [];
let requests: ProjectFilter[] = [];

beforeEach(() => {
  requests = [];
  projects = [
    project("p1", "Аудит", { status: "lead", deadline: "2024-05-01" }),
    project("p2", "Поддержка", {
      clientId: "c2",
      clientName: "Василёк",
      billingType: "hourly",
      currency: "EUR",
      rateMinor: 5_000,
      budgetMinor: null,
    }),
  ];
  mockIPC((cmd, args) => {
    if (cmd === "list_projects") {
      requests.push((args as { filter: ProjectFilter }).filter);
      return projects;
    }
    if (cmd === "list_clients") return [client("c1", "Ромашка"), client("c2", "Василёк")];
  });
});

afterEach(() => clearMocks());

const rows = () => screen.getAllByRole("row").slice(1);
const lastRequest = () => requests.at(-1);

async function choose(label: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name: label }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

describe("ProjectList", () => {
  it("renders projects with client, status, price and deadline", async () => {
    render(ProjectList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    const [audit, support] = rows();
    expect(within(audit).getByRole("link", { name: "Аудит" })).toHaveAttribute(
      "href",
      "/projects/p1",
    );
    expect(within(audit).getByText("Ромашка")).toBeInTheDocument();
    expect(within(audit).getByText("Lead")).toBeInTheDocument();
    expect(within(audit).getByText(/100,000\.00/)).toBeInTheDocument();
    expect(within(audit).getByText("May 1, 2024")).toBeInTheDocument();
    expect(within(support).getByText(/€50\.00\/h/)).toBeInTheDocument();
    expect(lastRequest()).toEqual({ query: null, status: null, clientId: null, archived: false });
  });

  it("filters by status, client, search and archive", async () => {
    render(ProjectList, { searchDelay: 0 });
    await waitFor(() => expect(rows()).toHaveLength(2));
    await choose("Status", "Completed");
    await waitFor(() => expect(lastRequest()?.status).toBe("completed"));
    await choose("Client", "Василёк");
    await waitFor(() => expect(lastRequest()?.clientId).toBe("c2"));
    await userEvent.type(screen.getByRole("textbox", { name: "Search projects" }), "под");
    await waitFor(() => expect(lastRequest()?.query).toBe("под"));
    await userEvent.click(screen.getByRole("switch", { name: "Show archived" }));
    await waitFor(() => expect(lastRequest()?.archived).toBe(true));
  });

  it("is limited to one client inside a client card", async () => {
    render(ProjectList, { clientId: "c1" });
    await waitFor(() => expect(lastRequest()?.clientId).toBe("c1"));
    expect(screen.queryByRole("combobox", { name: "Client" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Client" })).not.toBeInTheDocument();
  });

  it("shows empty states", async () => {
    projects = [];
    render(ProjectList);
    expect(await screen.findByText("No projects yet")).toBeInTheDocument();
    await choose("Status", "Lead");
    expect(await screen.findByText("No projects match the filters")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(ProjectList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
