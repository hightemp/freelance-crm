import type { CoreError, ErrorParam } from "./bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CoreError };

export class AppError extends Error {
  constructor(
    readonly code: string,
    readonly params: Record<string, ErrorParam> = {},
  ) {
    super(code);
    this.name = "AppError";
  }
}

/** Unwraps a generated command result, throwing `AppError` on failure. */
export async function call<T>(result: Promise<Result<T>>): Promise<T> {
  const res = await result;
  if (res.status === "ok") return res.data;
  if (isCoreError(res.error)) throw new AppError(res.error.code, res.error.params);
  throw new AppError("internal.unknown");
}

function isCoreError(value: unknown): value is CoreError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as CoreError).code === "string" &&
    typeof (value as CoreError).params === "object"
  );
}
