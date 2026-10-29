//! iCalendar (RFC 5545) reading: VEVENT with RRULE, EXDATE, RECURRENCE-ID
//! and VALARM. Times keep their IANA zone; Windows zone names (Outlook)
//! are mapped, other unknown zones use the VTIMEZONE's X-LIC-LOCATION or
//! the local zone with a warning. Broken events are skipped and reported.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use serde::Serialize;

use super::rrule::Rrule;
use crate::CoreError;

/// An event as read from a file, in [`super::events::EventInput`] terms.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IcsEvent {
    pub uid: String,
    pub title: String,
    pub description: String,
    pub location: Option<String>,
    pub all_day: bool,
    pub start: String,
    pub end: String,
    pub tzid: String,
    pub rrule: Option<String>,
    pub exdates: Vec<String>,
    /// For an edited occurrence of a series: which one.
    pub recurrence_id: Option<String>,
    /// Display and audio alarms before the start, in minutes.
    pub reminders: Vec<u32>,
}

/// A skipped event or a doubt about one.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IcsIssue {
    pub uid: Option<String>,
    /// 1-based line of the event's BEGIN:VEVENT.
    pub line: u32,
    pub error: CoreError,
    /// The event was imported anyway.
    pub warning: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ParsedIcs {
    pub events: Vec<IcsEvent>,
    pub issues: Vec<IcsIssue>,
}

/// Reads a calendar; floating times and dates use `local_tz`.
pub fn parse(text: &str, local_tz: &str) -> ParsedIcs {
    let lines = unfold(text);
    let mut zones: Vec<Component> = Vec::new();
    let mut events: Vec<Component> = Vec::new();
    let mut issues = Vec::new();
    // Components being read, innermost last.
    let mut stack: Vec<Component> = Vec::new();
    for line in &lines {
        let Some(prop) = Property::parse(&line.text) else {
            continue;
        };
        match prop.name.as_str() {
            "BEGIN" => stack.push(Component {
                name: prop.value.to_ascii_uppercase(),
                line: line.number,
                props: Vec::new(),
                children: Vec::new(),
            }),
            "END" => {
                let Some(done) = stack.pop() else { continue };
                match (done.name.as_str(), stack.last_mut()) {
                    ("VEVENT", _) => events.push(done),
                    ("VTIMEZONE", _) => zones.push(done),
                    (_, Some(parent)) => parent.children.push(done),
                    _ => {}
                }
            }
            _ => {
                if let Some(current) = stack.last_mut() {
                    current.props.push(prop);
                }
            }
        }
    }
    for open in stack.iter().filter(|c| c.name == "VEVENT") {
        issues.push(IcsIssue {
            uid: open.value("UID").map(str::to_owned),
            line: open.line,
            error: CoreError::new("ics.unterminated"),
            warning: false,
        });
    }
    let zones = Zones {
        zones,
        local: local_tz.to_owned(),
    };
    let mut out = Vec::new();
    for component in &events {
        let mut warnings = Vec::new();
        match read_event(component, &zones, &mut warnings) {
            Ok(event) => out.push(event),
            Err(error) => issues.push(IcsIssue {
                uid: component.value("UID").map(str::to_owned),
                line: component.line,
                error,
                warning: false,
            }),
        }
        issues.extend(warnings.into_iter().map(|error| IcsIssue {
            uid: component.value("UID").map(str::to_owned),
            line: component.line,
            error,
            warning: true,
        }));
    }
    issues.sort_by_key(|i| i.line);
    ParsedIcs {
        events: out,
        issues,
    }
}

struct Line {
    number: u32,
    text: String,
}

/// Joins folded lines (a CRLF or LF followed by a space or tab).
fn unfold(text: &str) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = raw.strip_prefix([' ', '\t'])
            && let Some(last) = out.last_mut()
        {
            last.text.push_str(rest);
            continue;
        }
        if !raw.is_empty() {
            out.push(Line {
                number: i as u32 + 1,
                text: raw.to_owned(),
            });
        }
    }
    out
}

#[derive(Debug, Clone)]
struct Property {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Property {
    /// `NAME;PARAM=value;PARAM="quoted":value`.
    fn parse(line: &str) -> Option<Self> {
        let mut quoted = false;
        let mut split = None;
        for (i, c) in line.char_indices() {
            match c {
                '"' => quoted = !quoted,
                ':' if !quoted => {
                    split = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let split = split?;
        let (head, value) = (&line[..split], &line[split + 1..]);
        let mut parts = head.split(';');
        let name = parts.next()?.trim().to_ascii_uppercase();
        let params = parts
            .filter_map(|p| p.split_once('='))
            .map(|(k, v)| {
                (
                    k.trim().to_ascii_uppercase(),
                    v.trim().trim_matches('"').to_owned(),
                )
            })
            .collect();
        Some(Self {
            name,
            params,
            value: value.to_owned(),
        })
    }

    fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
struct Component {
    name: String,
    line: u32,
    props: Vec<Property>,
    children: Vec<Component>,
}

impl Component {
    fn prop(&self, name: &str) -> Option<&Property> {
        self.props.iter().find(|p| p.name == name)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.prop(name).map(|p| p.value.as_str())
    }

    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Property> {
        self.props.iter().filter(move |p| p.name == name)
    }
}

/// Windows zone names used by Outlook and Exchange.
const WINDOWS_ZONES: &[(&str, &str)] = &[
    ("Russian Standard Time", "Europe/Moscow"),
    ("Kaliningrad Standard Time", "Europe/Kaliningrad"),
    ("Ekaterinburg Standard Time", "Asia/Yekaterinburg"),
    ("N. Central Asia Standard Time", "Asia/Novosibirsk"),
    ("North Asia Standard Time", "Asia/Krasnoyarsk"),
    ("Vladivostok Standard Time", "Asia/Vladivostok"),
    ("GMT Standard Time", "Europe/London"),
    ("W. Europe Standard Time", "Europe/Berlin"),
    ("Romance Standard Time", "Europe/Paris"),
    ("Central Europe Standard Time", "Europe/Budapest"),
    ("Central European Standard Time", "Europe/Warsaw"),
    ("E. Europe Standard Time", "Europe/Chisinau"),
    ("FLE Standard Time", "Europe/Kiev"),
    ("GTB Standard Time", "Europe/Bucharest"),
    ("Turkey Standard Time", "Europe/Istanbul"),
    ("Eastern Standard Time", "America/New_York"),
    ("Central Standard Time", "America/Chicago"),
    ("Mountain Standard Time", "America/Denver"),
    ("Pacific Standard Time", "America/Los_Angeles"),
    ("Singapore Standard Time", "Asia/Singapore"),
    ("China Standard Time", "Asia/Shanghai"),
    ("Tokyo Standard Time", "Asia/Tokyo"),
    ("Korea Standard Time", "Asia/Seoul"),
    ("India Standard Time", "Asia/Kolkata"),
    ("SE Asia Standard Time", "Asia/Bangkok"),
    ("Arabian Standard Time", "Asia/Dubai"),
    ("AUS Eastern Standard Time", "Australia/Sydney"),
    ("UTC", "UTC"),
];

struct Zones {
    zones: Vec<Component>,
    local: String,
}

impl Zones {
    /// An IANA name for a TZID, with a warning when it had to be guessed.
    fn resolve(&self, tzid: &str, warnings: &mut Vec<CoreError>) -> String {
        if TimeZone::get(tzid).is_ok() {
            return tzid.to_owned();
        }
        if let Some((_, iana)) = WINDOWS_ZONES.iter().find(|(w, _)| *w == tzid) {
            return (*iana).to_owned();
        }
        let location = self
            .zones
            .iter()
            .find(|z| z.value("TZID") == Some(tzid))
            .and_then(|z| z.value("X-LIC-LOCATION"))
            .filter(|l| TimeZone::get(l).is_ok());
        if let Some(location) = location {
            return location.to_owned();
        }
        let warning = CoreError::new("ics.unknown_timezone").with("tzid", tzid.to_owned());
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
        self.local.clone()
    }
}

/// A DTSTART-like value: a date, or a wall-clock time in an IANA zone.
enum When {
    Date(Date),
    Time(DateTime, String),
}

fn invalid_date() -> CoreError {
    CoreError::new("ics.invalid_date")
}

/// Reads a date or date-time value (one of a comma list).
fn when(
    prop: &Property,
    value: &str,
    zones: &Zones,
    warnings: &mut Vec<CoreError>,
) -> Result<When, CoreError> {
    let value = value.trim();
    if prop
        .param("VALUE")
        .is_some_and(|v| v.eq_ignore_ascii_case("DATE"))
        || value.len() == 8
    {
        return Date::strptime("%Y%m%d", value)
            .map(When::Date)
            .map_err(|_| invalid_date());
    }
    if let Some(utc) = value.strip_suffix(['Z', 'z']) {
        let dt = DateTime::strptime("%Y%m%dT%H%M%S", utc).map_err(|_| invalid_date())?;
        return Ok(When::Time(dt, "UTC".into()));
    }
    let dt = DateTime::strptime("%Y%m%dT%H%M%S", value).map_err(|_| invalid_date())?;
    let tz = match prop.param("TZID") {
        Some(tzid) => zones.resolve(tzid, warnings),
        None => zones.local.clone(),
    };
    Ok(When::Time(dt, tz))
}

/// The wall-clock time of `when` in zone `tz`.
fn in_zone(when: &When, tz: &str) -> Option<DateTime> {
    match when {
        When::Date(d) => Some(d.to_datetime(jiff::civil::Time::midnight())),
        When::Time(dt, zone) if zone == tz => Some(*dt),
        When::Time(dt, zone) => {
            let instant = TimeZone::get(zone)
                .ok()?
                .to_ambiguous_zoned(*dt)
                .compatible()
                .ok()?;
            Some(instant.with_time_zone(TimeZone::get(tz).ok()?).datetime())
        }
    }
}

/// `[+-]P[nW][nD][T[nH][nM][nS]]` as a signed span.
fn duration(value: &str) -> Option<jiff::Span> {
    let value = value.trim();
    let (negative, rest) = match value.as_bytes().first()? {
        b'-' => (true, &value[1..]),
        b'+' => (false, &value[1..]),
        _ => (false, value),
    };
    let rest = rest.strip_prefix(['P', 'p'])?;
    // Weeks, days, hours, minutes, seconds.
    let mut parts = [0i64; 5];
    let (mut in_time, mut number) = (false, String::new());
    for c in rest.chars() {
        match c.to_ascii_uppercase() {
            '0'..='9' => number.push(c),
            'T' => in_time = true,
            unit => {
                let n: i64 = number.parse().ok()?;
                number.clear();
                let slot = match (unit, in_time) {
                    ('W', false) => 0,
                    ('D', false) => 1,
                    ('H', true) => 2,
                    ('M', true) => 3,
                    ('S', true) => 4,
                    _ => return None,
                };
                parts[slot] += n;
            }
        }
    }
    if !number.is_empty() {
        return None;
    }
    let [weeks, days, hours, minutes, seconds] = parts;
    let span = jiff::Span::new()
        .try_weeks(weeks)
        .ok()?
        .try_days(days)
        .ok()?
        .try_hours(hours)
        .ok()?
        .try_minutes(minutes)
        .ok()?
        .try_seconds(seconds)
        .ok()?;
    Some(if negative { span.negate() } else { span })
}

/// Undoes RFC 5545 TEXT escaping.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Minutes before the start of display and audio alarms.
fn alarms(event: &Component) -> Vec<u32> {
    let mut out: Vec<u32> = event
        .children
        .iter()
        .filter(|c| c.name == "VALARM")
        .filter(|c| {
            c.value("ACTION").is_some_and(|a| {
                a.eq_ignore_ascii_case("DISPLAY") || a.eq_ignore_ascii_case("AUDIO")
            })
        })
        .filter_map(|c| {
            let trigger = c.prop("TRIGGER")?;
            let related_end = trigger
                .param("RELATED")
                .is_some_and(|r| r.eq_ignore_ascii_case("END"));
            let absolute = trigger
                .param("VALUE")
                .is_some_and(|v| v.eq_ignore_ascii_case("DATE-TIME"));
            if related_end || absolute {
                return None;
            }
            let span = duration(&trigger.value)?;
            let minutes = span
                .total((jiff::Unit::Minute, jiff::civil::date(2000, 1, 1)))
                .ok()?;
            (minutes <= 0.0).then(|| (-minutes).round() as u32)
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn read_event(
    c: &Component,
    zones: &Zones,
    warnings: &mut Vec<CoreError>,
) -> Result<IcsEvent, CoreError> {
    let start_prop = c
        .prop("DTSTART")
        .ok_or_else(|| CoreError::new("ics.no_start"))?;
    let start = when(start_prop, &start_prop.value, zones, warnings)?;
    let (all_day, tzid) = match &start {
        When::Date(_) => (true, zones.local.clone()),
        When::Time(_, tz) => (false, tz.clone()),
    };
    let start_local = in_zone(&start, &tzid).ok_or_else(invalid_date)?;
    let end_local = if let Some(end) = c.prop("DTEND") {
        let end = when(end, &end.value, zones, warnings)?;
        in_zone(&end, &tzid).ok_or_else(invalid_date)?
    } else if let Some(d) = c.value("DURATION") {
        let span = duration(d).ok_or_else(invalid_date)?;
        start_local.checked_add(span).map_err(|_| invalid_date())?
    } else if all_day {
        start_local
            .checked_add(jiff::Span::new().days(1))
            .map_err(|_| invalid_date())?
    } else {
        start_local
    };
    if end_local < start_local {
        return Err(CoreError::validation("end", "before_start"));
    }
    let text = |dt: DateTime| {
        if all_day {
            dt.date().to_string()
        } else {
            super::events::format_local(dt)
        }
    };
    let end_text = if all_day {
        // DTEND of all-day events is exclusive; ours is the last day.
        let last = end_local.date().yesterday().map_err(|_| invalid_date())?;
        last.max(start_local.date()).to_string()
    } else {
        text(end_local)
    };
    let rrule = c
        .value("RRULE")
        .map(|r| r.parse::<Rrule>().map(|r| r.to_string()))
        .transpose()?;
    let mut exdates = Vec::new();
    for prop in c.all("EXDATE") {
        for value in prop.value.split(',') {
            let at = when(prop, value, zones, warnings)?;
            exdates.push(text(in_zone(&at, &tzid).ok_or_else(invalid_date)?));
        }
    }
    exdates.sort();
    exdates.dedup();
    let recurrence_id = c
        .prop("RECURRENCE-ID")
        .map(|p| {
            when(p, &p.value, zones, warnings)
                .and_then(|w| in_zone(&w, &tzid).ok_or_else(invalid_date))
                .map(text)
        })
        .transpose()?;
    let location = c
        .value("LOCATION")
        .map(unescape)
        .filter(|l| !l.trim().is_empty());
    Ok(IcsEvent {
        uid: c
            .value("UID")
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}@import", crate::domain::Id::new())),
        title: c
            .value("SUMMARY")
            .map(unescape)
            .unwrap_or_default()
            .trim()
            .to_owned(),
        description: c.value("DESCRIPTION").map(unescape).unwrap_or_default(),
        location,
        all_day,
        start: text(start_local),
        end: end_text,
        tzid,
        rrule,
        exdates,
        recurrence_id,
        reminders: alarms(c),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOGLE: &str = include_str!("../../tests/fixtures/ics/google.ics");
    const APPLE: &str = include_str!("../../tests/fixtures/ics/apple.ics");
    const BROKEN: &str = include_str!("../../tests/fixtures/ics/broken.ics");

    fn event(uid: &str, title: &str, start: &str, end: &str, tzid: &str) -> IcsEvent {
        IcsEvent {
            uid: uid.into(),
            title: title.into(),
            description: String::new(),
            location: None,
            all_day: false,
            start: start.into(),
            end: end.into(),
            tzid: tzid.into(),
            rrule: None,
            exdates: Vec::new(),
            recurrence_id: None,
            reminders: Vec::new(),
        }
    }

    #[test]
    fn reads_a_google_export() {
        let parsed = parse(GOOGLE, "Asia/Kuala_Lumpur");
        assert_eq!(parsed.issues, []);
        assert_eq!(
            parsed.events,
            [
                IcsEvent {
                    description:
                        "Короткий созвон, статус по задачам.\nСсылка: https://meet.example/abc"
                            .into(),
                    location: Some("Zoom".into()),
                    rrule: Some("FREQ=WEEKLY;BYDAY=MO".into()),
                    exdates: vec!["2026-03-16T10:00".into()],
                    reminders: vec![10],
                    ..event(
                        "standup-1@google.com",
                        "Планёрка",
                        "2026-03-02T10:00",
                        "2026-03-02T10:30",
                        "Europe/Moscow"
                    )
                },
                IcsEvent {
                    recurrence_id: Some("2026-03-09T10:00".into()),
                    ..event(
                        "standup-1@google.com",
                        "Планёрка (перенос)",
                        "2026-03-10T12:00",
                        "2026-03-10T12:30",
                        "Europe/Moscow"
                    )
                },
                IcsEvent {
                    all_day: true,
                    ..event(
                        "holidays@google.com",
                        "Майские",
                        "2026-05-01",
                        "2026-05-03",
                        "Asia/Kuala_Lumpur"
                    )
                },
                // Folded lines are joined; e-mail and end-related alarms are skipped.
                event(
                    "utc-call@google.com",
                    "Call with a very long title that will certainly be folded by the exporter \
                     because lines are limited to 75 octets",
                    "2026-03-20T15:00",
                    "2026-03-20T16:00",
                    "UTC"
                ),
            ]
        );
    }

    #[test]
    fn reads_an_apple_export_and_maps_zones() {
        let parsed = parse(APPLE, "Asia/Kuala_Lumpur");
        assert_eq!(parsed.issues, []);
        assert_eq!(
            parsed.events,
            [
                IcsEvent {
                    rrule: Some("FREQ=MONTHLY;COUNT=6;BYDAY=1FR".into()),
                    reminders: vec![60, 2880],
                    ..event(
                        "6F3C1A2B-1111-2222-3333-444455556666",
                        "Client sync",
                        "2026-03-06T09:00",
                        "2026-03-06T10:00",
                        "America/New_York"
                    )
                },
                event(
                    "outlook-style",
                    "Outlook meeting",
                    "2026-03-11T09:00",
                    "2026-03-11T10:00",
                    "Europe/Moscow"
                ),
                // Floating time with a DURATION.
                event(
                    "floating",
                    "Плавающее",
                    "2026-03-12T08:00",
                    "2026-03-12T08:45",
                    "Asia/Kuala_Lumpur"
                ),
            ]
        );
    }

    #[test]
    fn broken_files_import_partially_with_a_report() {
        let parsed = parse(BROKEN, "UTC");
        assert_eq!(
            parsed
                .events
                .iter()
                .map(|e| e.uid.as_str())
                .collect::<Vec<_>>(),
            ["ok-1", "ok-2"]
        );
        // Without DTEND: a moment, or a single day.
        assert_eq!(parsed.events[0].end, "2026-04-01T10:00");
        assert_eq!(parsed.events[1].end, "2026-04-05");
        assert_eq!(
            parsed
                .issues
                .iter()
                .map(|i| (i.uid.as_deref(), i.line, i.error.code.as_str(), i.warning))
                .collect::<Vec<_>>(),
            [
                (Some("no-start"), 8, "ics.no_start", false),
                (Some("bad-date"), 12, "ics.invalid_date", false),
                (Some("bad-rule"), 17, "rrule.unsupported", false),
                (Some("truncated"), 28, "ics.unterminated", false),
            ]
        );
    }

    #[test]
    fn unknown_zones_fall_back_with_a_warning() {
        let text = "BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nTZID:My Zone\r\nX-LIC-LOCATION:Europe/Berlin\r\n\
                    END:VTIMEZONE\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=My Zone:20260310T100000\r\n\
                    END:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART;TZID=\"Mars/Base\":20260310T100000\r\n\
                    END:VEVENT\r\nEND:VCALENDAR\r\n";
        let parsed = parse(text, "Asia/Tokyo");
        assert_eq!(parsed.events[0].tzid, "Europe/Berlin");
        assert_eq!(parsed.events[1].tzid, "Asia/Tokyo");
        assert_eq!(parsed.issues.len(), 1);
        assert!(parsed.issues[0].warning);
        assert_eq!(
            parsed.issues[0].error,
            CoreError::new("ics.unknown_timezone").with("tzid", "Mars/Base")
        );
    }

    #[test]
    fn accepts_lf_line_ends_and_lowercase_and_utc_exdates() {
        let text = "begin:vcalendar\nbegin:vevent\nuid:x\ndtstart;tzid=Europe/Moscow:20260302T100000\n\
                    rrule:FREQ=DAILY;COUNT=3\nexdate:20260303T070000Z\nsummary:a\\;b\\\\c\nend:vevent\n\
                    end:vcalendar\n";
        let parsed = parse(text, "UTC");
        assert_eq!(parsed.issues, []);
        let e = &parsed.events[0];
        assert_eq!(e.title, "a;b\\c");
        // A UTC EXDATE is turned into the event's local time.
        assert_eq!(e.exdates, ["2026-03-03T10:00"]);
    }
}
