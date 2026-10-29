import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type {
  ClientFilter_Deserialize as ClientFilter,
  Client_Serialize as Client,
  Tag,
} from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import ClientList from "./ClientList.svelte";

const DAY = 86_400_000;
const T0 = Date.UTC(2024, 0, 15);

function client(name: string, patch: Partial<Client> = {}): Client {
  return {
    id: `id-${name}`,
    kind: "company",
    lastName: null,
    firstName: null,
    middleName: null,
    companyName: name,
    legalName: null,
    status: "active",
    defaultCurrency: null,
    defaultRateMinor: null,
    displayName: name,
    tagIds: [],
    createdAt: T0,
    updatedAt: T0,
    archivedAt: null,
    ...patch,
  };
}

const TAGS: Tag[] = [
  { id: "t-vip", name: "VIP", color: "#ef4444", createdAt: T0 },
  { id: "t-urgent", name: "Срочно", color: "#3b82f6", createdAt: T0 },
];

let clients: Client[] = [];
let requests: ClientFilter[] = [];

beforeEach(() => {
  requests = [];
  clients = [
    client("Ромашка", {
      tagIds: ["t-vip"],
      defaultCurrency: "RUB",
      defaultRateMinor: 250_000,
      updatedAt: T0 + DAY,
    }),
    client("Иванов Иван", {
      kind: "person",
      lastName: "Иванов",
      firstName: "Иван",
      companyName: null,
      status: "lead",
    }),
  ];
  mockIPC((cmd, args) => {
    if (cmd === "list_clients") {
      requests.push((args as { filter: ClientFilter }).filter);
      return clients;
    }
    if (cmd === "list_tags") return TAGS;
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const rows = () => screen.getAllByRole("row").slice(1); // skip the header row
const names = () => rows().map((row) => within(row).getAllByRole("cell")[0].textContent?.trim());
const lastRequest = () => requests.at(-1);

describe("ClientList", () => {
  it("renders clients with status, tags and rate", async () => {
    render(ClientList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    const [ivanov, romashka] = rows(); // sorted by name
    expect(within(romashka).getByText("Ромашка")).toBeInTheDocument();
    expect(within(romashka).getByText("Active")).toBeInTheDocument();
    expect(within(romashka).getByText("VIP")).toBeInTheDocument();
    expect(within(romashka).getByText(/2,500\.00/)).toBeInTheDocument();
    expect(within(ivanov).getByText("Lead")).toBeInTheDocument();
    expect(lastRequest()).toEqual({ query: null, status: null, archived: false, tagIds: [] });
  });

  it("debounces search input into a single request", async () => {
    render(ClientList, { searchDelay: 50 });
    await waitFor(() => expect(requests).toHaveLength(1));
    await userEvent.type(screen.getByRole("textbox", { name: "Search clients" }), "ром");
    await waitFor(() => expect(lastRequest()?.query).toBe("ром"));
    expect(requests.map((r) => r.query)).toEqual([null, "ром"]);
  });

  it("filters by status", async () => {
    render(ClientList);
    await userEvent.click(screen.getByRole("combobox", { name: "Status" }));
    await userEvent.click(screen.getByRole("option", { name: "Lead" }));
    await waitFor(() => expect(lastRequest()?.status).toBe("lead"));
  });

  it("filters by tags and archived", async () => {
    render(ClientList);
    await userEvent.click(screen.getByRole("button", { name: "Tags" }));
    await userEvent.click(await screen.findByRole("checkbox", { name: "Срочно" }));
    await waitFor(() => expect(lastRequest()?.tagIds).toEqual(["t-urgent"]));
    expect(screen.getByRole("button", { name: "Tags: 1" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("switch", { name: "Show archived" }));
    await waitFor(() => expect(lastRequest()?.archived).toBe(true));
  });

  it("sorts by column", async () => {
    render(ClientList);
    await waitFor(() => expect(names()).toEqual(["Иванов Иван", "Ромашка"]));
    const nameHeader = screen.getByRole("columnheader", { name: /Name/ });
    expect(nameHeader).toHaveAttribute("aria-sort", "ascending");

    await userEvent.click(within(nameHeader).getByRole("button"));
    expect(names()).toEqual(["Ромашка", "Иванов Иван"]);
    expect(nameHeader).toHaveAttribute("aria-sort", "descending");

    const updated = screen.getByRole("columnheader", { name: /Updated/ });
    await userEvent.click(within(updated).getByRole("button"));
    expect(names()).toEqual(["Иванов Иван", "Ромашка"]);
    expect(nameHeader).not.toHaveAttribute("aria-sort");
  });

  it("shows an empty state when there are no clients", async () => {
    clients = [];
    render(ClientList);
    expect(await screen.findByText("No clients yet")).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });

  it("offers to reset filters when nothing matches", async () => {
    render(ClientList, { searchDelay: 0 });
    await waitFor(() => expect(rows()).toHaveLength(2));
    clients = [];
    await userEvent.type(screen.getByRole("textbox", { name: "Search clients" }), "zzz");
    expect(await screen.findByText("No clients match the filters")).toBeInTheDocument();

    clients = [client("Ромашка")];
    await userEvent.click(screen.getByRole("button", { name: "Reset filters" }));
    await waitFor(() => expect(lastRequest()?.query).toBeNull());
    expect(screen.getByRole("textbox", { name: "Search clients" })).toHaveValue("");
  });

  it("is translated", async () => {
    setAppLocale("ru");
    render(ClientList);
    expect(await screen.findByRole("textbox", { name: "Поиск клиентов" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Лид")).toBeInTheDocument());
  });

  it("has no a11y violations", async () => {
    const { container } = render(ClientList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
