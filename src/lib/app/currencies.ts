/** Currencies offered in pickers; any ISO 4217 code is valid in the backend. */
export const COMMON_CURRENCIES = [
  "RUB",
  "USD",
  "EUR",
  "GBP",
  "MYR",
  "KZT",
  "BYN",
  "UAH",
  "GEL",
  "AMD",
  "TRY",
  "AED",
  "CNY",
  "JPY",
  "SGD",
  "THB",
  "INR",
  "CAD",
  "AUD",
  "CHF",
  "PLN",
];

export function currencyLabel(code: string, locale: string): string {
  const name = new Intl.DisplayNames(locale, { type: "currency" }).of(code);
  return name && name !== code ? `${code} — ${name}` : code;
}
