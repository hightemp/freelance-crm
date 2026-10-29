// Pure layout of calendar items: side-by-side columns for overlapping
// events in a day, and lanes of (multi-)day items across a week row.

export interface Span {
  id: string;
  /** Minutes from the start of the day (may exceed a day). */
  start: number;
  end: number;
}

export interface Placement {
  column: number;
  /** Columns in the overlapping cluster the event belongs to. */
  columns: number;
}

/** Columns for overlapping events: each cluster of overlapping events is
 * split into as many columns as it needs at most, reusing freed ones. */
export function layoutDay(items: Span[]): Map<string, Placement> {
  // A zero-length event still occupies a sliver.
  const end = (s: Span) => Math.max(s.end, s.start + 1);
  const sorted = [...items].sort((a, b) => a.start - b.start || end(b) - end(a));
  const out = new Map<string, Placement>();
  let cluster: { id: string; column: number }[] = [];
  let clusterEnd = -Infinity;
  /** End of the event last placed in each column. */
  let columnEnds: number[] = [];

  const close = () => {
    const columns = columnEnds.length;
    for (const { id, column } of cluster) out.set(id, { column, columns });
    cluster = [];
    columnEnds = [];
  };

  for (const item of sorted) {
    if (item.start >= clusterEnd) close();
    let column = columnEnds.findIndex((e) => e <= item.start);
    if (column === -1) column = columnEnds.push(0) - 1;
    columnEnds[column] = end(item);
    cluster.push({ id: item.id, column });
    clusterEnd = Math.max(clusterEnd, end(item));
  }
  close();
  return out;
}

export interface DaySpan {
  id: string;
  /** `YYYY-MM-DD`, inclusive. */
  start: string;
  end: string;
}

export interface Segment {
  id: string;
  /** Day of the week row, 0–6. */
  column: number;
  span: number;
  lane: number;
  continuesBefore: boolean;
  continuesAfter: boolean;
}

/** The parts of day items inside a week row (`week`: its 7 dates), in
 * lanes: earlier first, longer first on the same day. */
export function weekSegments(items: DaySpan[], week: string[]): Segment[] {
  const first = week[0];
  const last = week[week.length - 1];
  const pieces = items
    .filter((i) => i.start <= last && i.end >= first)
    .map((i) => {
      const from = i.start < first ? first : i.start;
      const to = i.end > last ? last : i.end;
      const column = week.indexOf(from);
      return {
        id: i.id,
        column,
        span: week.indexOf(to) - column + 1,
        continuesBefore: i.start < first,
        continuesAfter: i.end > last,
      };
    })
    .sort((a, b) => a.column - b.column || b.span - a.span);
  /** Lanes taken per day of the week. */
  const taken: Set<number>[] = week.map(() => new Set());
  return pieces.map((p) => {
    const days = taken.slice(p.column, p.column + p.span);
    let lane = 0;
    while (days.some((d) => d.has(lane))) lane++;
    for (const d of days) d.add(lane);
    return { ...p, lane };
  });
}

/** Per day of the week: how many items are below the `visible` lanes. */
export function hiddenPerDay(segments: Segment[], visible: number, days = 7): number[] {
  const hidden: number[] = Array.from({ length: days }, () => 0);
  for (const s of segments) {
    if (s.lane < visible) continue;
    for (let d = s.column; d < s.column + s.span; d++) hidden[d]++;
  }
  return hidden;
}
