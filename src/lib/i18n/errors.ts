import { AppError } from "$lib/api/client";
import { m } from "$lib/paraglide/messages";

type MessageFn = (params?: Record<string, unknown>) => string;

/**
 * Translates a backend error code (`money.currency_mismatch`) via the
 * message `error_money_currency_mismatch`, falling back to a generic text.
 */
export function errorMessage(err: unknown): string {
  const appError = err instanceof AppError ? err : new AppError("internal.unknown");
  return codeMessage(appError.code, appError.params);
}

/** Translates an error code, e.g. a form validation code like `validation.email`. */
export function codeMessage(code: string, params: Record<string, unknown> = {}): string {
  const key = `error_${code.replaceAll(".", "_")}`;
  const message = (m as unknown as Record<string, MessageFn | undefined>)[key];
  return message ? message(params) : m.error_unknown({ code });
}
