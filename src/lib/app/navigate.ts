import { goto } from "$app/navigation";

/**
 * Navigates to an already resolved path with query parameters; `resolve()`
 * has no query support.
 */
export function gotoWithQuery(path: string, query: Record<string, string>): Promise<void> {
  const search = new URLSearchParams(query).toString();
  // eslint-disable-next-line svelte/no-navigation-without-resolve -- path is resolved by the caller
  return goto(search ? `${path}?${search}` : path);
}
