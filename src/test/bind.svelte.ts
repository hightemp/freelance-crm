/**
 * Props with one reactive `bind:`-able key, for testing `$bindable` props.
 * Reads go through `$state`, so components re-render on changes.
 */
export function bind<T, P extends object>(props: P, key: string, initial: T) {
  const state = $state({ value: initial });
  const all = { ...props };
  Object.defineProperty(all, key, {
    enumerable: true,
    configurable: true,
    get: () => state.value,
    set: (v: T) => (state.value = v),
  });
  return { state, props: all as P & Record<string, T> };
}
