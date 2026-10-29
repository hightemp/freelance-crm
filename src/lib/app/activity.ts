const EVENTS = ["pointerdown", "keydown", "wheel"] as const;

/**
 * Calls `report` on user input, at most once per `intervalMs`; used to keep
 * the vault from auto-locking while the user works. Returns an uninstaller.
 */
export function installActivityReporter(report: () => void, intervalMs: number): () => void {
  let last = -Infinity;
  const onActivity = () => {
    const now = Date.now();
    if (now - last < intervalMs) return;
    last = now;
    report();
  };
  for (const name of EVENTS) window.addEventListener(name, onActivity, { passive: true });
  return () => {
    for (const name of EVENTS) window.removeEventListener(name, onActivity);
  };
}
