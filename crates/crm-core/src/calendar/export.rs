//! iCalendar export (FR-CA-8): all events, a client's or a project's, or
//! those in a date range. Times keep their IANA zone, described by a
//! VTIMEZONE with the zone's transitions around the exported events; UTC
//! times are written with `Z`. Lines are folded at 75 octets.

use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::events::{Event, EventRepo, Series};
use super::reminders::ReminderRepo;
use crate::CoreResult;
use crate::domain::{Clock, EntityRef, Id, Timestamp};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportFilter {
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
    /// Events overlapping `[from, to)`.
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}

/// The calendar file text.
pub fn export(conn: &Connection, clock: &dyn Clock, filter: ExportFilter) -> CoreResult<String> {
    let events = EventRepo::new(conn, clock);
    let reminders = ReminderRepo::new(conn, clock);
    let mut stmt = conn.prepare(
        "SELECT id FROM events
         WHERE series_id IS NULL
           AND (?1 IS NULL OR client_id = ?1)
           AND (?2 IS NULL OR project_id = ?2)
           AND (?4 IS NULL OR start_at < ?4)
           AND (?3 IS NULL
                OR (rrule IS NULL AND (end_at > ?3 OR start_at = ?3))
                OR (rrule IS NOT NULL AND (series_end_at IS NULL OR series_end_at > ?3)))
         ORDER BY start_at, id",
    )?;
    let ids = stmt
        .query_map(
            rusqlite::params![filter.client_id, filter.project_id, filter.from, filter.to],
            |r| r.get::<_, Id>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut overrides =
        conn.prepare("SELECT id FROM events WHERE series_id = ?1 ORDER BY recurrence_id")?;

    let mut body = Vec::new();
    let mut zones: Vec<(String, Timestamp, Timestamp)> = Vec::new();
    let stamp = format_utc(clock.now());
    for id in ids {
        let event = events.get(id)?;
        let alarms: Vec<u32> = reminders
            .list(EntityRef::Event(id))?
            .iter()
            .map(|r| r.minutes_before)
            .collect();
        note_zone(&mut zones, &event);
        write_event(&mut body, &event, None, &alarms, &stamp);
        for over in overrides.query_map([id], |r| r.get::<_, Id>(0))? {
            let over = events.get(over?)?;
            note_zone(&mut zones, &over);
            write_event(&mut body, &over, Some(&event), &[], &stamp);
        }
    }

    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&fold(&text));
        out.push_str("\r\n");
    };
    line("BEGIN:VCALENDAR".into());
    line("VERSION:2.0".into());
    line("PRODID:-//freelance-crm//freelance-crm//EN".into());
    line("CALSCALE:GREGORIAN".into());
    for (tzid, from, to) in &zones {
        for l in vtimezone(tzid, *from, *to) {
            line(l);
        }
    }
    for l in body {
        line(l);
    }
    line("END:VCALENDAR".into());
    Ok(out)
}

/// Years of zone transitions written around the exported events, and how
/// far an endless series is assumed to reach.
const ZONE_MARGIN_MS: i64 = 366 * 24 * 3_600_000;
const ENDLESS_MS: i64 = 10 * 366 * 24 * 3_600_000;

/// Widens the period a zone must describe to cover `event`.
fn note_zone(zones: &mut Vec<(String, Timestamp, Timestamp)>, event: &Event) {
    if event.all_day || event.tzid == "UTC" {
        return;
    }
    let from = event.start_at.as_millis() - ZONE_MARGIN_MS;
    let to = match (&event.rrule, series_end(event)) {
        (Some(_), Some(end)) => end.as_millis(),
        (Some(_), None) => event.start_at.as_millis() + ENDLESS_MS,
        (None, _) => event.end_at.as_millis(),
    } + ZONE_MARGIN_MS;
    match zones.iter_mut().find(|(tz, _, _)| *tz == event.tzid) {
        Some((_, start, end)) => {
            *start = Timestamp::from_millis(start.as_millis().min(from));
            *end = Timestamp::from_millis(end.as_millis().max(to));
        }
        None => zones.push((
            event.tzid.clone(),
            Timestamp::from_millis(from),
            Timestamp::from_millis(to),
        )),
    }
}

fn series_end(event: &Event) -> Option<Timestamp> {
    Series::from_event(event).and_then(|s| s.end_at(event.end_at))
}

/// A VTIMEZONE with the transitions of `tzid` in `[from, to]`.
fn vtimezone(tzid: &str, from: Timestamp, to: Timestamp) -> Vec<String> {
    let Ok(tz) = TimeZone::get(tzid) else {
        return Vec::new();
    };
    let ts = |t: Timestamp| jiff::Timestamp::from_millisecond(t.as_millis()).unwrap_or_default();
    let (from, to) = (ts(from), ts(to));
    let mut lines = vec![
        "BEGIN:VTIMEZONE".to_owned(),
        format!("TZID:{tzid}"),
        format!("X-LIC-LOCATION:{tzid}"),
    ];
    let observance = |lines: &mut Vec<String>,
                      dst: bool,
                      start: DateTime,
                      before: Offset,
                      after: Offset,
                      name: &str| {
        let kind = if dst { "DAYLIGHT" } else { "STANDARD" };
        lines.push(format!("BEGIN:{kind}"));
        lines.push(format!("DTSTART:{}", start.strftime("%Y%m%dT%H%M%S")));
        lines.push(format!("TZOFFSETFROM:{}", format_offset(before)));
        lines.push(format!("TZOFFSETTO:{}", format_offset(after)));
        if !name.is_empty() {
            lines.push(format!("TZNAME:{name}"));
        }
        lines.push(format!("END:{kind}"));
    };
    // The rules in force at the start of the period.
    let info = tz.to_offset_info(from);
    let first = info.offset();
    observance(
        &mut lines,
        info.dst().is_dst(),
        first.to_datetime(from),
        first,
        first,
        info.abbreviation(),
    );
    let mut before = first;
    for t in tz.following(from).take_while(|t| t.timestamp() <= to) {
        // Observances start at the local time of the old offset.
        observance(
            &mut lines,
            t.dst().is_dst(),
            before.to_datetime(t.timestamp()),
            before,
            t.offset(),
            t.abbreviation(),
        );
        before = t.offset();
    }
    lines.push("END:VTIMEZONE".to_owned());
    lines
}

/// `+HHMM`, with seconds only when there are any.
fn format_offset(offset: Offset) -> String {
    let total = offset.seconds();
    let sign = if total < 0 { '-' } else { '+' };
    let total = total.unsigned_abs();
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if s == 0 {
        format!("{sign}{h:02}{m:02}")
    } else {
        format!("{sign}{h:02}{m:02}{s:02}")
    }
}

fn format_utc(at: Timestamp) -> String {
    jiff::Timestamp::from_millisecond(at.as_millis())
        .unwrap_or_default()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string()
}

/// `;VALUE=DATE:20260501`, `:20260320T150000Z` or `;TZID=…:20260302T100000`
/// for a local value of `event`.
fn value(event: &Event, local: &str) -> String {
    if event.all_day {
        return format!(";VALUE=DATE:{}", local.replace('-', ""));
    }
    let compact = format!("{}00", local.replace(['-', ':'], ""));
    if event.tzid == "UTC" {
        format!(":{compact}Z")
    } else {
        format!(";TZID={}:{compact}", event.tzid)
    }
}

fn write_event(
    out: &mut Vec<String>,
    event: &Event,
    series: Option<&Event>,
    alarms: &[u32],
    stamp: &str,
) {
    out.push("BEGIN:VEVENT".into());
    out.push(format!("UID:{}", series.map_or(&event.uid, |s| &s.uid)));
    out.push(format!("DTSTAMP:{stamp}"));
    out.push(format!("DTSTART{}", value(event, &event.start)));
    let end = if event.all_day {
        // Inclusive last day → exclusive DTEND.
        event
            .end
            .parse::<jiff::civil::Date>()
            .ok()
            .and_then(|d| d.tomorrow().ok())
            .map_or_else(|| event.end.clone(), |d| d.to_string())
    } else {
        event.end.clone()
    };
    out.push(format!("DTEND{}", value(event, &end)));
    if let (Some(series), Some(rid)) = (series, &event.recurrence_id) {
        out.push(format!("RECURRENCE-ID{}", value(series, rid)));
    }
    if let Some(rule) = &event.rrule {
        out.push(format!("RRULE:{rule}"));
        if !event.exdates.is_empty() {
            let first = value(event, &event.exdates[0]);
            let (head, _) = first.rsplit_once(':').expect("values have a colon");
            let rest: Vec<String> = event
                .exdates
                .iter()
                .map(|x| {
                    value(event, x)
                        .rsplit_once(':')
                        .expect("colon")
                        .1
                        .to_owned()
                })
                .collect();
            out.push(format!("EXDATE{head}:{}", rest.join(",")));
        }
    }
    out.push(format!("SUMMARY:{}", escape(&event.title)));
    if !event.description_md.is_empty() {
        out.push(format!("DESCRIPTION:{}", escape(&event.description_md)));
    }
    if let Some(location) = &event.location {
        out.push(format!("LOCATION:{}", escape(location)));
    }
    for minutes in alarms {
        out.push("BEGIN:VALARM".into());
        out.push("ACTION:DISPLAY".into());
        out.push(format!("DESCRIPTION:{}", escape(&event.title)));
        out.push(format!("TRIGGER:-PT{minutes}M"));
        out.push("END:VALARM".into());
    }
    out.push("END:VEVENT".into());
}

/// RFC 5545 TEXT escaping.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Splits a content line into physical lines of at most 75 octets,
/// continuation lines starting with a space, never inside a character.
fn fold(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + line.len() / 70 * 3);
    let mut width = 0;
    for c in line.chars() {
        let len = c.len_utf8();
        if width + len > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::events::{EditScope, EventInput, EventRepo};
    use crate::calendar::ics::parse;
    use crate::calendar::import::{ImportLink, import};
    use crate::calendar::reminders::ReminderRepo;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{EntityRef, FakeClock};

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    fn input(title: &str, start: &str, end: &str, tzid: &str) -> EventInput {
        EventInput {
            title: title.into(),
            description_md: String::new(),
            location: None,
            all_day: false,
            start: start.into(),
            end: end.into(),
            tzid: tzid.into(),
            client_id: None,
            project_id: None,
            task_id: None,
            contact_id: None,
            rrule: None,
        }
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
    }

    impl Fixture {
        /// A varied calendar: a Moscow weekly series with an exception,
        /// an edited occurrence and a reminder; an all-day event; a UTC
        /// call with awkward text; a New York monthly series for a client.
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(utc("2026-03-01T00:00:00Z"));
            let client = ClientRepo::new(db.conn(), &clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some("Ромашка".into()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id;
            let f = Self { db, clock, client };
            let events = EventRepo::new(f.db.conn(), &f.clock);
            let mut weekly = input(
                "Планёрка",
                "2026-03-02T10:00",
                "2026-03-02T10:30",
                "Europe/Moscow",
            );
            weekly.rrule = Some("FREQ=WEEKLY;BYDAY=MO".into());
            weekly.description_md = "Статус; задачи, риски\nи \\планы".into();
            let weekly = events.create(weekly).unwrap();
            events
                .delete_occurrence(weekly.id, "2026-03-16T10:00", EditScope::This)
                .unwrap();
            events
                .update_occurrence(
                    weekly.id,
                    "2026-03-09T10:00",
                    EditScope::This,
                    input(
                        "Планёрка (перенос)",
                        "2026-03-10T12:00",
                        "2026-03-10T12:30",
                        "Europe/Moscow",
                    ),
                )
                .unwrap();
            ReminderRepo::new(f.db.conn(), &f.clock)
                .set(EntityRef::Event(weekly.id), &[10, 1440])
                .unwrap();
            let mut holidays = input("Майские", "2026-05-01", "2026-05-03", "Europe/Moscow");
            holidays.all_day = true;
            events.create(holidays).unwrap();
            let mut call = input(
                "Очень длинное название звонка, которое точно придётся переносить по правилам",
                "2026-03-20T15:00",
                "2026-03-20T16:00",
                "UTC",
            );
            call.location = Some("Офис, 3 этаж; переговорная".into());
            events.create(call).unwrap();
            let mut sync = input(
                "Client sync",
                "2026-03-06T09:00",
                "2026-03-06T10:00",
                "America/New_York",
            );
            sync.rrule = Some("FREQ=MONTHLY;COUNT=6;BYDAY=1FR".into());
            sync.client_id = Some(f.client);
            events.create(sync).unwrap();
            f
        }

        fn export(&self, filter: ExportFilter) -> String {
            export(self.db.conn(), &self.clock, filter).unwrap()
        }
    }

    /// Everything a user sees: occurrences of 2026 by start, with their
    /// zone and reminders of the series.
    fn snapshot(db: &Db, clock: &FakeClock) -> Vec<String> {
        let reminders = ReminderRepo::new(db.conn(), clock);
        EventRepo::new(db.conn(), clock)
            .occurrences(utc("2026-01-01T00:00:00Z"), utc("2027-01-01T00:00:00Z"))
            .unwrap()
            .into_iter()
            .map(|o| {
                let series = o.event.series_id.unwrap_or(o.event.id);
                let minutes: Vec<u32> = reminders
                    .list(EntityRef::Event(series))
                    .unwrap()
                    .iter()
                    .map(|r| r.minutes_before)
                    .collect();
                format!(
                    "{} {}–{} {} {:?} {:?} {:?} {:?} {:?}",
                    o.event.title,
                    o.start,
                    o.end,
                    // All-day events are floating dates: no zone to keep.
                    if o.event.all_day {
                        "-"
                    } else {
                        o.event.tzid.as_str()
                    },
                    o.event.all_day,
                    o.event.description_md,
                    o.event.location,
                    o.recurrence_id,
                    minutes
                )
            })
            .collect()
    }

    #[test]
    fn export_then_import_keeps_everything() {
        let f = Fixture::new();
        let text = f.export(ExportFilter::default());
        let fresh = Db::open_in_memory().unwrap();
        let parsed = parse(&text, "Asia/Tokyo");
        assert_eq!(parsed.issues, []);
        let report = import(fresh.conn(), &f.clock, &parsed, None, ImportLink::default()).unwrap();
        assert_eq!((report.created, report.updated), (5, 0));
        let before = snapshot(&f.db, &f.clock);
        // 44 Mondays from March 2, less the removed and the moved one, plus
        // the moved one; the holidays; the call; six monthly syncs.
        assert_eq!(before.len(), 43 + 1 + 1 + 6);
        assert_eq!(snapshot(&fresh, &f.clock), before);
    }

    #[test]
    fn output_follows_rfc_5545() {
        let f = Fixture::new();
        let text = f.export(ExportFilter::default());
        assert!(text.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:"));
        assert!(text.ends_with("END:VCALENDAR\r\n"));
        assert!(
            !text.replace("\r\n", "").contains('\n'),
            "only CRLF line ends"
        );
        for line in text.split("\r\n") {
            assert!(line.len() <= 75, "{line:?} is {} octets", line.len());
        }
        // Folding never splits a character: each physical line is UTF-8.
        let unfolded = text.replace("\r\n ", "");
        assert!(unfolded.contains(
            r"SUMMARY:Очень длинное название звонка\, которое точно придётся переносить по правилам"
        ));
        assert!(unfolded.contains(r"DESCRIPTION:Статус\; задачи\, риски\nи \\планы"));
        assert!(unfolded.contains(r"LOCATION:Офис\, 3 этаж\; переговорная"));
        assert!(unfolded.contains("DTSTART:20260320T150000Z"));
        assert!(unfolded.contains("DTSTART;VALUE=DATE:20260501\r\nDTEND;VALUE=DATE:20260504"));
        assert!(unfolded.contains("DTSTART;TZID=Europe/Moscow:20260302T100000"));
        assert!(unfolded.contains("EXDATE;TZID=Europe/Moscow:20260309T100000,20260316T100000"));
        assert!(unfolded.contains("RECURRENCE-ID;TZID=Europe/Moscow:20260309T100000"));
        assert!(unfolded.contains("TRIGGER:-PT1440M"));
        assert_eq!(unfolded.matches("BEGIN:VTIMEZONE").count(), 2);
        // New York switches to summer time on 2026-03-08 at 02:00.
        assert!(unfolded.contains(
            "BEGIN:DAYLIGHT\r\nDTSTART:20260308T020000\r\nTZOFFSETFROM:-0500\r\nTZOFFSETTO:-0400\r\nTZNAME:EDT\r\nEND:DAYLIGHT"
        ));
        assert!(unfolded.contains("X-LIC-LOCATION:Europe/Moscow"));
        assert!(unfolded.matches("UID:").count() >= 5);
    }

    fn summaries(text: &str) -> Vec<String> {
        parse(text, "UTC")
            .events
            .into_iter()
            .map(|e| e.title)
            .collect()
    }

    #[test]
    fn filters_by_client_and_range() {
        let f = Fixture::new();
        assert_eq!(
            summaries(&f.export(ExportFilter {
                client_id: Some(f.client),
                ..Default::default()
            })),
            ["Client sync"]
        );
        // May: the holidays and both still running series with their exception.
        assert_eq!(
            summaries(&f.export(ExportFilter {
                from: Some(utc("2026-05-01T00:00:00Z")),
                to: Some(utc("2026-06-01T00:00:00Z")),
                ..Default::default()
            })),
            ["Планёрка", "Планёрка (перенос)", "Client sync", "Майские"]
        );
        assert!(
            summaries(&f.export(ExportFilter {
                project_id: Some(Id::new()),
                ..Default::default()
            }))
            .is_empty()
        );
    }
}
