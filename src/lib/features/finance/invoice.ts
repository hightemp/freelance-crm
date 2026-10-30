import type { Discount, Invoice_Serialize as Invoice, InvoiceStatus } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

/** `value * num / den` rounded half to even, as the core does it. */
export function scaled(value: number, num: number, den: number): number {
  const exact = BigInt(value) * BigInt(num);
  const d = BigInt(den);
  let q = exact / d;
  let r = exact % d;
  // Floor division, so that the remainder is never negative.
  if (r < 0n) {
    q -= 1n;
    r += d;
  }
  const twice = r * 2n;
  if (twice > d || (twice === d && q % 2n !== 0n)) q += 1n;
  return Number(q);
}

export interface Totals {
  lines: number[];
  subtotal: number;
  discount: number;
  tax: number;
  total: number;
  paid: number;
  due: number;
}

/** Live totals of an invoice being edited, in minor units. */
export function computeTotals(
  items: { price: number; quantityMilli: number }[],
  discount: Discount,
  taxBp: number,
  paid: number,
): Totals {
  const lines = items.map((i) => scaled(i.price, i.quantityMilli, 1000));
  const subtotal = lines.reduce((a, b) => a + b, 0);
  const off =
    discount.type === "percent"
      ? scaled(subtotal, discount.value, 10_000)
      : discount.type === "fixed"
        ? Math.min(discount.value, subtotal)
        : 0;
  const tax = scaled(subtotal - off, taxBp, 10_000);
  const total = subtotal - off + tax;
  return { lines, subtotal, discount: off, tax, total, paid, due: total - paid };
}

/** As in the core: an invoice paid even in part, or cancelled, is final. */
export function isLocked(invoice: Pick<Invoice, "storedStatus" | "totals">): boolean {
  return invoice.storedStatus === "cancelled" || invoice.totals.paid.amountMinor > 0;
}

export function statusLabel(status: InvoiceStatus): string {
  switch (status) {
    case "draft":
      return m.invoice_status_draft();
    case "sent":
      return m.invoice_status_sent();
    case "partially_paid":
      return m.invoice_status_partially_paid();
    case "paid":
      return m.invoice_status_paid();
    case "overdue":
      return m.invoice_status_overdue();
    case "cancelled":
      return m.invoice_status_cancelled();
  }
}
