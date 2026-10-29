import { convertFileSrc } from "@tauri-apps/api/core";

/** URL of an attachment image served by the app's `crmfile://` protocol. */
export function previewUrl(kind: "thumb" | "full", id: string): string {
  return convertFileSrc(`${kind}/${id}`, "crmfile");
}

/** "2026-10-29 14-03-07": a local timestamp safe for file names. */
export function fileStamp(date: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}-${p(date.getMonth() + 1)}-${p(date.getDate())} ` +
    `${p(date.getHours())}-${p(date.getMinutes())}-${p(date.getSeconds())}`
  );
}
