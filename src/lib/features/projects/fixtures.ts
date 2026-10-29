// Test data shared by project component tests.
import type { Client_Serialize as Client, Project_Serialize as Project } from "$lib/api/bindings";

export function client(id: string, name: string, patch: Partial<Client> = {}): Client {
  return {
    id,
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
    createdAt: 0,
    updatedAt: 0,
    archivedAt: null,
    ...patch,
  };
}

export function project(id: string, name: string, patch: Partial<Project> = {}): Project {
  return {
    id,
    clientId: "c1",
    clientName: "Ромашка",
    name,
    descriptionMd: "",
    status: "active",
    billingType: "fixed",
    currency: "RUB",
    rateMinor: null,
    budgetMinor: 10_000_000,
    startDate: null,
    deadline: null,
    createdAt: 0,
    updatedAt: 0,
    archivedAt: null,
    ...patch,
  };
}
