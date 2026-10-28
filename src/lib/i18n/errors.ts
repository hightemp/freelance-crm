import { AppError } from "$lib/api/client";
import { m } from "$lib/paraglide/messages";

type MessageFn = (params?: Record<string, unknown>) => string;

/**
 * Translates a backend error code (`money.currency_mismatch`) via the
 * message `error_money_currency_mismatch`, falling back to a generic text.
 */
export function errorMessage(err: unknown): string {
  const appError = err instanceof AppError ? err : new AppError("internal.unknown");
  const key = `error_${appError.code.replaceAll(".", "_")}`;
  const message = (m as unknown as Record<string, MessageFn | undefined>)[key];
  return message ? message(appError.params) : m.error_unknown({ code: appError.code });
}
