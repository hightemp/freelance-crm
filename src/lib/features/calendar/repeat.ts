// Repeat presets of the event form and the RRULEs they stand for. Rules a
// preset cannot express are kept and edited as text.

const WEEKDAYS = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

export type Ends =
  { kind: "never" } | { kind: "count"; count: number } | { kind: "until"; date: string };

export type Preset = "none" | "daily" | "weekdays" | "weekly" | "monthly" | "yearly" | "custom";

export interface Repeat {
  preset: Preset;
  /** The rule itself for `custom` (its own COUNT/UNTIL included). */
  rule?: string;
  ends: Ends;
}

function dayParts(date: string) {
  const [y, m, d] = date.split("-").map(Number);
  return { weekday: WEEKDAYS[new Date(Date.UTC(y, m - 1, d)).getUTCDay()], day: d };
}

/** The base rule of a preset for an event starting on `date`. */
function base(preset: Preset, date: string): string | null {
  const { weekday, day } = dayParts(date);
  switch (preset) {
    case "daily":
      return "FREQ=DAILY";
    case "weekdays":
      return "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR";
    case "weekly":
      return `FREQ=WEEKLY;BYDAY=${weekday}`;
    case "monthly":
      return `FREQ=MONTHLY;BYMONTHDAY=${day}`;
    case "yearly":
      return "FREQ=YEARLY";
    default:
      return null;
  }
}

/** The RRULE for the form's choice, `null` for a single event. */
export function buildRule(repeat: Repeat, startDate: string): string | null {
  if (repeat.preset === "none") return null;
  if (repeat.preset === "custom") return repeat.rule?.trim() || null;
  const rule = base(repeat.preset, startDate)!;
  switch (repeat.ends.kind) {
    case "never":
      return rule;
    case "count":
      return `${rule};COUNT=${repeat.ends.count}`;
    case "until":
      return `${rule};UNTIL=${repeat.ends.date.replaceAll("-", "")}`;
  }
}

/** The form's choice for a stored (canonical) rule. */
export function readRule(rule: string | null, startDate: string): Repeat {
  const never: Ends = { kind: "never" };
  if (!rule) return { preset: "none", ends: never };
  const parts = rule.split(";");
  let ends: Ends = never;
  const rest = parts.filter((p) => {
    const [name, value] = p.split("=");
    if (name === "COUNT") ends = { kind: "count", count: Number(value) };
    else if (name === "UNTIL" && /^\d{8}$/.test(value))
      ends = { kind: "until", date: `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6)}` };
    else return true;
    return false;
  });
  const core = rest.join(";");
  for (const preset of ["daily", "weekdays", "weekly", "monthly", "yearly"] as const) {
    // A weekly rule without BYDAY repeats on the start's weekday.
    if (core === base(preset, startDate) || (preset === "weekly" && core === "FREQ=WEEKLY"))
      return { preset, ends };
  }
  return { preset: "custom", rule, ends: never };
}
