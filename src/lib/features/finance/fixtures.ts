import type {
  Client_Serialize as Client,
  Invoice_Serialize as Invoice,
  Project_Serialize as Project,
} from "$lib/api/bindings";

/** Test data shared by the finance specs. */
export const CLIENTS = [
  { id: "c1", displayName: "Ромашка", defaultCurrency: "RUB" },
  { id: "c2", displayName: "Acme", defaultCurrency: "USD" },
] as unknown as Client[];

export const PROJECTS = [
  { id: "p1", name: "Сайт", clientId: "c1", clientName: "Ромашка", currency: "RUB" },
] as unknown as Project[];

const rub = (amountMinor: number) => ({ amountMinor, currency: "RUB" });

export function invoice(patch: Partial<Invoice> = {}): Invoice {
  return {
    id: "i1",
    number: "INV-2026-001",
    clientId: "c1",
    clientName: "Ромашка",
    projectId: "p1",
    projectName: "Сайт",
    currency: "RUB",
    issueDate: "2026-03-01",
    dueDate: "2026-03-15",
    storedStatus: "sent",
    status: "sent",
    items: [
      {
        id: "it1",
        description: "Вёрстка",
        quantityMilli: 2000,
        unit: "ч",
        price: rub(150000),
        amount: rub(300000),
      },
    ],
    discount: { type: "none" },
    taxBp: 0,
    notesMd: "",
    language: "ru",
    showSignature: null,
    totals: {
      subtotal: rub(300000),
      discount: rub(0),
      tax: rub(0),
      total: rub(300000),
      paid: rub(0),
      due: rub(300000),
    },
    createdAt: 1,
    updatedAt: 1,
    ...patch,
  };
}
