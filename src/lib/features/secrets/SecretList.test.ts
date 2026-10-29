import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type {
  SecretFilter_Deserialize as SecretFilter,
  Secret_Serialize as Secret,
} from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { secret } from "./fixtures";
import SecretList from "./SecretList.svelte";

let secrets: Secret[] = [];
let requests: SecretFilter[] = [];

beforeEach(() => {
  requests = [];
  secrets = [
    secret("s1", "Хостинг", { clientName: "Ромашка", clientId: "c1", projectName: "Сайт" }),
    secret("s2", "Deploy key", { kind: "ssh_key", expiresOn: "2025-03-01" }),
  ];
  mockIPC((cmd, args) => {
    if (cmd === "list_secrets") {
      requests.push((args as { filter: SecretFilter }).filter);
      return secrets;
    }
  });
});

afterEach(() => clearMocks());

const items = () => screen.getAllByRole("listitem");

describe("SecretList", () => {
  it("lists secrets without values", async () => {
    render(SecretList);
    await waitFor(() => expect(items()).toHaveLength(2));
    const [hosting, key] = items();
    expect(within(hosting).getByRole("link", { name: "Хостинг" })).toHaveAttribute(
      "href",
      "/vault/s1",
    );
    expect(within(hosting).getByText("Login")).toBeInTheDocument();
    expect(within(hosting).getByText("Ромашка · Сайт")).toBeInTheDocument();
    expect(within(key).getByText("SSH key")).toBeInTheDocument();
    expect(within(key).getByText("Renews on Mar 1, 2025")).toBeInTheDocument();
  });

  it("searches and limits to a client or project", async () => {
    render(SecretList, { clientId: "c1", searchDelay: 0 });
    await waitFor(() => expect(requests.at(-1)?.clientId).toBe("c1"));
    await userEvent.type(screen.getByRole("textbox", { name: "Search secrets" }), "хост");
    await waitFor(() => expect(requests.at(-1)?.query).toBe("хост"));
    expect(requests.at(-1)).toEqual({ query: "хост", kind: null, clientId: "c1", projectId: null });
  });

  it("shows empty states", async () => {
    secrets = [];
    render(SecretList, { searchDelay: 0 });
    expect(await screen.findByText("No secrets yet")).toBeInTheDocument();
    await userEvent.type(screen.getByRole("textbox", { name: "Search secrets" }), "x");
    expect(await screen.findByText("No secrets match the search")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(SecretList);
    await waitFor(() => expect(items()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
