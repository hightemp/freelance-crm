//! RFC 5545 recurrence rules (FR-CA-4): the subset people and calendar
//! apps use — FREQ (DAILY, WEEKLY, MONTHLY, YEARLY), INTERVAL, COUNT,
//! UNTIL, BYDAY (with ordinals such as `2TU` or `-1FR`), BYMONTHDAY,
//! BYMONTH, BYSETPOS and WKST. Other parts are rejected as unsupported.
//!
//! Occurrences are generated in local wall-clock time, so a 10:00 event
//! stays at 10:00 across DST changes.

use std::fmt;
use std::str::FromStr;

use jiff::civil::{Date, DateTime, Weekday};
use jiff::tz::TimeZone;

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// A BYDAY entry: a weekday, optionally the n-th in the month or year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByDay {
    pub nth: Option<i8>,
    pub weekday: Weekday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    /// A floating date (all-day events): the last day included.
    Date(Date),
    /// `YYYYMMDDTHHMMSSZ`: an instant.
    Instant(jiff::Timestamp),
    /// `YYYYMMDDTHHMMSS`: local time of the event.
    Local(DateTime),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rrule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<Until>,
    pub by_day: Vec<ByDay>,
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<i8>,
    pub by_set_pos: Vec<i16>,
    pub wkst: Weekday,
}

impl FromStr for Rrule {
    type Err = CoreError;

    /// Accepts an optional `RRULE:` prefix, any case and order of parts.
    fn from_str(s: &str) -> CoreResult<Self> {
        let s = s.trim();
        let s = s
            .get(..6)
            .filter(|p| p.eq_ignore_ascii_case("RRULE:"))
            .map_or(s, |_| &s[6..]);
        let mut freq = None;
        let mut rule = Rrule {
            freq: Freq::Daily,
            interval: 1,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_set_pos: Vec::new(),
            wkst: Weekday::Monday,
        };
        for part in s.split(';').filter(|p| !p.trim().is_empty()) {
            let (name, value) = part.split_once('=').ok_or_else(|| invalid(part))?;
            let (name, value) = (
                name.trim().to_ascii_uppercase(),
                value.trim().to_ascii_uppercase(),
            );
            match name.as_str() {
                "FREQ" => {
                    freq = Some(match value.as_str() {
                        "DAILY" => Freq::Daily,
                        "WEEKLY" => Freq::Weekly,
                        "MONTHLY" => Freq::Monthly,
                        "YEARLY" => Freq::Yearly,
                        "SECONDLY" | "MINUTELY" | "HOURLY" => return Err(unsupported(&name)),
                        _ => return Err(invalid(&name)),
                    })
                }
                "INTERVAL" => {
                    rule.interval = value
                        .parse()
                        .ok()
                        .filter(|&n| n > 0)
                        .ok_or_else(|| invalid(&name))?
                }
                "COUNT" => {
                    rule.count = Some(
                        value
                            .parse()
                            .ok()
                            .filter(|&n| n > 0)
                            .ok_or_else(|| invalid(&name))?,
                    )
                }
                "UNTIL" => rule.until = Some(parse_until(&value).ok_or_else(|| invalid(&name))?),
                "BYDAY" => {
                    rule.by_day = list(&value, parse_by_day).ok_or_else(|| invalid(&name))?
                }
                "BYMONTHDAY" => {
                    rule.by_month_day = list(&value, |v| {
                        v.parse::<i8>()
                            .ok()
                            .filter(|n| *n != 0 && (-31..=31).contains(n))
                    })
                    .ok_or_else(|| invalid(&name))?
                }
                "BYMONTH" => {
                    rule.by_month = list(&value, |v| {
                        v.parse::<i8>().ok().filter(|n| (1..=12).contains(n))
                    })
                    .ok_or_else(|| invalid(&name))?
                }
                "BYSETPOS" => {
                    rule.by_set_pos = list(&value, |v| {
                        v.parse::<i16>()
                            .ok()
                            .filter(|n| *n != 0 && (-366..=366).contains(n))
                    })
                    .ok_or_else(|| invalid(&name))?
                }
                "WKST" => rule.wkst = parse_weekday(&value).ok_or_else(|| invalid(&name))?,
                "BYSECOND" | "BYMINUTE" | "BYHOUR" | "BYYEARDAY" | "BYWEEKNO" | "RSCALE"
                | "SKIP" => return Err(unsupported(&name)),
                _ => return Err(invalid(&name)),
            }
        }
        rule.freq = freq.ok_or_else(|| invalid("FREQ"))?;
        if rule.count.is_some() && rule.until.is_some() {
            return Err(invalid("UNTIL"));
        }
        let has_nth = rule.by_day.iter().any(|d| d.nth.is_some());
        match rule.freq {
            Freq::Daily | Freq::Weekly if has_nth => return Err(invalid("BYDAY")),
            Freq::Yearly if has_nth && rule.by_month.is_empty() => {
                return Err(unsupported("BYDAY"));
            }
            _ => {}
        }
        Ok(rule)
    }
}

fn invalid(part: &str) -> CoreError {
    CoreError::new("rrule.invalid").with("part", part.to_owned())
}

fn unsupported(part: &str) -> CoreError {
    CoreError::new("rrule.unsupported").with("part", part.to_owned())
}

fn list<T>(value: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    value.split(',').map(|v| parse(v.trim())).collect()
}

const WEEKDAYS: [(&str, Weekday); 7] = [
    ("MO", Weekday::Monday),
    ("TU", Weekday::Tuesday),
    ("WE", Weekday::Wednesday),
    ("TH", Weekday::Thursday),
    ("FR", Weekday::Friday),
    ("SA", Weekday::Saturday),
    ("SU", Weekday::Sunday),
];

fn parse_weekday(v: &str) -> Option<Weekday> {
    WEEKDAYS
        .iter()
        .find(|(code, _)| *code == v)
        .map(|(_, wd)| *wd)
}

fn weekday_code(wd: Weekday) -> &'static str {
    WEEKDAYS
        .iter()
        .find(|(_, w)| *w == wd)
        .map(|(c, _)| *c)
        .expect("all weekdays listed")
}

fn parse_by_day(v: &str) -> Option<ByDay> {
    let split = v.len().checked_sub(2)?;
    let weekday = parse_weekday(v.get(split..)?)?;
    let nth = match &v[..split] {
        "" => None,
        n => Some(
            n.trim_start_matches('+')
                .parse::<i8>()
                .ok()
                .filter(|n| *n != 0 && (-53..=53).contains(n))?,
        ),
    };
    Some(ByDay { nth, weekday })
}

fn parse_until(v: &str) -> Option<Until> {
    if v.len() == 8 {
        return jiff::civil::Date::strptime("%Y%m%d", v)
            .ok()
            .map(Until::Date);
    }
    if let Some(local) = v.strip_suffix('Z') {
        let dt = DateTime::strptime("%Y%m%dT%H%M%S", local).ok()?;
        return Some(Until::Instant(dt.to_zoned(TimeZone::UTC).ok()?.timestamp()));
    }
    DateTime::strptime("%Y%m%dT%H%M%S", v)
        .ok()
        .map(Until::Local)
}

impl fmt::Display for Rrule {
    /// Canonical form: FREQ first, parts in a fixed order, defaults left out.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let freq = match self.freq {
            Freq::Daily => "DAILY",
            Freq::Weekly => "WEEKLY",
            Freq::Monthly => "MONTHLY",
            Freq::Yearly => "YEARLY",
        };
        write!(f, "FREQ={freq}")?;
        if self.interval != 1 {
            write!(f, ";INTERVAL={}", self.interval)?;
        }
        if let Some(count) = self.count {
            write!(f, ";COUNT={count}")?;
        }
        match self.until {
            Some(Until::Date(d)) => write!(f, ";UNTIL={}", d.strftime("%Y%m%d"))?,
            Some(Until::Instant(ts)) => write!(
                f,
                ";UNTIL={}",
                ts.to_zoned(TimeZone::UTC).strftime("%Y%m%dT%H%M%SZ")
            )?,
            Some(Until::Local(dt)) => write!(f, ";UNTIL={}", dt.strftime("%Y%m%dT%H%M%S"))?,
            None => {}
        }
        let join = |items: Vec<String>| items.join(",");
        if !self.by_day.is_empty() {
            let days = self
                .by_day
                .iter()
                .map(|d| {
                    format!(
                        "{}{}",
                        d.nth.map(|n| n.to_string()).unwrap_or_default(),
                        weekday_code(d.weekday)
                    )
                })
                .collect();
            write!(f, ";BYDAY={}", join(days))?;
        }
        if !self.by_month_day.is_empty() {
            write!(
                f,
                ";BYMONTHDAY={}",
                join(self.by_month_day.iter().map(|n| n.to_string()).collect())
            )?;
        }
        if !self.by_month.is_empty() {
            write!(
                f,
                ";BYMONTH={}",
                join(self.by_month.iter().map(|n| n.to_string()).collect())
            )?;
        }
        if !self.by_set_pos.is_empty() {
            write!(
                f,
                ";BYSETPOS={}",
                join(self.by_set_pos.iter().map(|n| n.to_string()).collect())
            )?;
        }
        if self.wkst != Weekday::Monday {
            write!(f, ";WKST={}", weekday_code(self.wkst))?;
        }
        Ok(())
    }
}

/// Periods in a row without any occurrence after which a rule is taken
/// to never match again (e.g. February 31st).
const MAX_EMPTY_PERIODS: u32 = 1000;

impl Rrule {
    /// Local start times of the series in order, from `dtstart` (always the
    /// first) up to COUNT or UNTIL; `tz` resolves an UNTIL instant.
    pub fn starts<'a>(&'a self, dtstart: DateTime, tz: &'a TimeZone) -> Starts<'a> {
        Starts {
            rule: self,
            tz,
            dtstart,
            period: 0,
            buffer: Vec::new(),
            emitted: 0,
            started: false,
            done: false,
        }
    }

    fn past_until(&self, at: DateTime, tz: &TimeZone) -> bool {
        match self.until {
            None => false,
            Some(Until::Date(d)) => at.date() > d,
            Some(Until::Local(u)) => at > u,
            Some(Until::Instant(u)) => tz
                .to_ambiguous_zoned(at)
                .compatible()
                .map_or(true, |z| z.timestamp() > u),
        }
    }

    /// Candidate dates of the `k`-th period (unsorted, unfiltered by dtstart).
    fn period_dates(&self, start: Date, k: i64) -> Option<Vec<Date>> {
        let step = k.checked_mul(i64::from(self.interval))?;
        let mut dates = match self.freq {
            Freq::Daily => {
                let date = start.checked_add(jiff::Span::new().days(step)).ok()?;
                let matches = (self.by_day.is_empty()
                    || self.by_day.iter().any(|d| d.weekday == date.weekday()))
                    && (self.by_month_day.is_empty()
                        || month_days(date.year(), date.month(), &self.by_month_day)
                            .contains(&date));
                if matches { vec![date] } else { Vec::new() }
            }
            Freq::Weekly => {
                let back = offset(start.weekday(), self.wkst);
                let week = start
                    .checked_sub(jiff::Span::new().days(back))
                    .ok()?
                    .checked_add(jiff::Span::new().weeks(step))
                    .ok()?;
                let weekdays: Vec<Weekday> = if self.by_day.is_empty() {
                    vec![start.weekday()]
                } else {
                    self.by_day.iter().map(|d| d.weekday).collect()
                };
                weekdays
                    .into_iter()
                    .filter_map(|wd| {
                        week.checked_add(jiff::Span::new().days(offset(wd, self.wkst)))
                            .ok()
                    })
                    .collect()
            }
            Freq::Monthly => {
                let months = i64::from(start.year()) * 12 + i64::from(start.month() - 1) + step;
                let (year, month) = (
                    i16::try_from(months.div_euclid(12)).ok()?,
                    (months.rem_euclid(12) + 1) as i8,
                );
                self.in_month(year, month, start.day())
            }
            Freq::Yearly => {
                let year = i16::try_from(i64::from(start.year()) + step).ok()?;
                let months = if self.by_month.is_empty() {
                    vec![start.month()]
                } else {
                    self.by_month.clone()
                };
                months
                    .into_iter()
                    .flat_map(|m| self.in_month(year, m, start.day()))
                    .collect()
            }
        };
        if !self.by_month.is_empty() {
            dates.retain(|d| self.by_month.contains(&d.month()));
        }
        dates.sort();
        dates.dedup();
        if !self.by_set_pos.is_empty() {
            let len = dates.len() as i64;
            let mut picked: Vec<Date> = self
                .by_set_pos
                .iter()
                .filter_map(|&p| {
                    let i = if p > 0 {
                        i64::from(p) - 1
                    } else {
                        len + i64::from(p)
                    };
                    usize::try_from(i).ok().and_then(|i| dates.get(i).copied())
                })
                .collect();
            picked.sort();
            picked.dedup();
            dates = picked;
        }
        Some(dates)
    }

    /// Days of a month matching BYMONTHDAY and BYDAY (the start's day if
    /// neither is given).
    fn in_month(&self, year: i16, month: i8, default_day: i8) -> Vec<Date> {
        let Ok(first) = Date::new(year, month, 1) else {
            return Vec::new();
        };
        let mut days = if self.by_month_day.is_empty() && self.by_day.is_empty() {
            Date::new(year, month, default_day).into_iter().collect()
        } else if self.by_month_day.is_empty() {
            Vec::new()
        } else {
            month_days(year, month, &self.by_month_day)
        };
        if !self.by_day.is_empty() {
            let all: Vec<Date> = (1..=first.days_in_month())
                .filter_map(|d| Date::new(year, month, d).ok())
                .collect();
            let by_day: Vec<Date> = self
                .by_day
                .iter()
                .flat_map(|bd| {
                    let same: Vec<Date> = all
                        .iter()
                        .copied()
                        .filter(|d| d.weekday() == bd.weekday)
                        .collect();
                    match bd.nth {
                        None => same,
                        Some(n) => {
                            let i = if n > 0 {
                                i64::from(n) - 1
                            } else {
                                same.len() as i64 + i64::from(n)
                            };
                            usize::try_from(i)
                                .ok()
                                .and_then(|i| same.get(i).copied())
                                .into_iter()
                                .collect()
                        }
                    }
                })
                .collect();
            days = if self.by_month_day.is_empty() {
                by_day
            } else {
                days.into_iter().filter(|d| by_day.contains(d)).collect()
            };
        }
        days
    }
}

/// Days from `from` forward to `wd`, 0..=6.
fn offset(wd: Weekday, from: Weekday) -> i64 {
    i64::from(wd.to_monday_zero_offset() - from.to_monday_zero_offset()).rem_euclid(7)
}

/// BYMONTHDAY values that exist in the month (negative count from the end).
fn month_days(year: i16, month: i8, days: &[i8]) -> Vec<Date> {
    let Ok(first) = Date::new(year, month, 1) else {
        return Vec::new();
    };
    let len = first.days_in_month();
    days.iter()
        .filter_map(|&d| {
            let day = if d > 0 { d } else { len + d + 1 };
            (1..=len)
                .contains(&day)
                .then(|| Date::new(year, month, day).ok())
                .flatten()
        })
        .collect()
}

pub struct Starts<'a> {
    rule: &'a Rrule,
    tz: &'a TimeZone,
    dtstart: DateTime,
    period: i64,
    /// The rest of the current period, latest first.
    buffer: Vec<DateTime>,
    emitted: u32,
    started: bool,
    done: bool,
}

impl Starts<'_> {
    fn emit(&mut self, at: DateTime) -> Option<DateTime> {
        if self.rule.past_until(at, self.tz) || self.rule.count.is_some_and(|c| self.emitted >= c) {
            self.done = true;
            return None;
        }
        self.emitted += 1;
        Some(at)
    }
}

impl Iterator for Starts<'_> {
    type Item = DateTime;

    fn next(&mut self) -> Option<DateTime> {
        if self.done {
            return None;
        }
        if !self.started {
            self.started = true;
            return self.emit(self.dtstart);
        }
        let mut empty = 0;
        loop {
            while let Some(at) = self.buffer.pop() {
                if at > self.dtstart {
                    return self.emit(at);
                }
            }
            let Some(dates) = self.rule.period_dates(self.dtstart.date(), self.period) else {
                self.done = true;
                return None;
            };
            self.period += 1;
            let time = self.dtstart.time();
            self.buffer = dates
                .into_iter()
                .rev()
                .map(|d| d.to_datetime(time))
                .collect();
            // Past UNTIL with nothing more to come.
            if let Some(last) = self.buffer.first()
                && self.rule.past_until(*last, self.tz)
                && self
                    .buffer
                    .iter()
                    .all(|d| self.rule.past_until(*d, self.tz))
            {
                self.done = true;
                return None;
            }
            if self.buffer.iter().all(|d| *d <= self.dtstart) {
                empty += 1;
                if empty > MAX_EMPTY_PERIODS {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn dt(s: &str) -> DateTime {
        s.parse().unwrap()
    }

    fn first(rule: &str, start: &str, n: usize) -> Vec<String> {
        let rule: Rrule = rule.parse().unwrap();
        rule.starts(dt(start), &TimeZone::UTC)
            .take(n)
            .map(|d| d.strftime("%Y-%m-%d %a %H:%M").to_string())
            .collect()
    }

    #[rstest]
    #[case::daily("FREQ=DAILY", "2026-01-30T09:00", 3,
        &["2026-01-30 Fri 09:00", "2026-01-31 Sat 09:00", "2026-02-01 Sun 09:00"])]
    #[case::every_other_day("FREQ=DAILY;INTERVAL=2", "2026-01-30T09:00", 3,
        &["2026-01-30 Fri 09:00", "2026-02-01 Sun 09:00", "2026-02-03 Tue 09:00"])]
    #[case::weekdays_daily("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR", "2026-01-30T09:00", 3,
        &["2026-01-30 Fri 09:00", "2026-02-02 Mon 09:00", "2026-02-03 Tue 09:00"])]
    #[case::weekly("FREQ=WEEKLY", "2026-01-28T10:00", 3,
        &["2026-01-28 Wed 10:00", "2026-02-04 Wed 10:00", "2026-02-11 Wed 10:00"])]
    #[case::weekly_byday("FREQ=WEEKLY;BYDAY=MO,WE,FR", "2026-01-28T10:00", 5,
        &["2026-01-28 Wed 10:00", "2026-01-30 Fri 10:00", "2026-02-02 Mon 10:00",
          "2026-02-04 Wed 10:00", "2026-02-06 Fri 10:00"])]
    #[case::biweekly_byday("FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH", "2026-01-27T10:00", 4,
        &["2026-01-27 Tue 10:00", "2026-01-29 Thu 10:00", "2026-02-10 Tue 10:00",
          "2026-02-12 Thu 10:00"])]
    #[case::monthly_31st_skips_short_months("FREQ=MONTHLY", "2026-01-31T12:00", 4,
        &["2026-01-31 Sat 12:00", "2026-03-31 Tue 12:00", "2026-05-31 Sun 12:00",
          "2026-07-31 Fri 12:00"])]
    #[case::monthly_last_day("FREQ=MONTHLY;BYMONTHDAY=-1", "2026-01-31T12:00", 3,
        &["2026-01-31 Sat 12:00", "2026-02-28 Sat 12:00", "2026-03-31 Tue 12:00"])]
    #[case::second_tuesday("FREQ=MONTHLY;BYDAY=2TU", "2026-01-13T18:00", 3,
        &["2026-01-13 Tue 18:00", "2026-02-10 Tue 18:00", "2026-03-10 Tue 18:00"])]
    #[case::last_friday("FREQ=MONTHLY;BYDAY=-1FR", "2026-01-30T18:00", 3,
        &["2026-01-30 Fri 18:00", "2026-02-27 Fri 18:00", "2026-03-27 Fri 18:00"])]
    #[case::last_workday("FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1", "2026-01-30T17:00", 3,
        &["2026-01-30 Fri 17:00", "2026-02-27 Fri 17:00", "2026-03-31 Tue 17:00"])]
    #[case::yearly_feb_29("FREQ=YEARLY", "2024-02-29T08:00", 3,
        &["2024-02-29 Thu 08:00", "2028-02-29 Tue 08:00", "2032-02-29 Sun 08:00"])]
    #[case::yearly_by_month_and_day("FREQ=YEARLY;BYMONTH=11;BYDAY=4TH", "2026-11-26T12:00", 2,
        &["2026-11-26 Thu 12:00", "2027-11-25 Thu 12:00"])]
    fn expands(#[case] rule: &str, #[case] start: &str, #[case] n: usize, #[case] want: &[&str]) {
        assert_eq!(first(rule, start, n), want);
    }

    #[test]
    fn dtstart_is_always_the_first_occurrence() {
        // A Wednesday start of a Monday-only rule still counts (RFC 5545).
        assert_eq!(
            first("FREQ=WEEKLY;BYDAY=MO;COUNT=2", "2026-01-28T10:00", 5),
            ["2026-01-28 Wed 10:00", "2026-02-02 Mon 10:00"]
        );
    }

    #[test]
    fn count_and_until_end_the_series() {
        assert_eq!(first("FREQ=DAILY;COUNT=2", "2026-01-01T09:00", 10).len(), 2);
        // UNTIL is inclusive.
        assert_eq!(
            first("FREQ=DAILY;UNTIL=20260103T090000Z", "2026-01-01T09:00", 10),
            [
                "2026-01-01 Thu 09:00",
                "2026-01-02 Fri 09:00",
                "2026-01-03 Sat 09:00"
            ]
        );
        assert_eq!(
            first("FREQ=WEEKLY;UNTIL=20260114", "2026-01-01T09:00", 10).len(),
            2
        );
        assert_eq!(
            first("FREQ=DAILY;UNTIL=20260102T085959", "2026-01-01T09:00", 10).len(),
            1
        );
    }

    #[test]
    fn until_instants_use_the_events_zone() {
        let rule: Rrule = "FREQ=DAILY;UNTIL=20260102T060000Z".parse().unwrap();
        let moscow = TimeZone::get("Europe/Moscow").unwrap();
        // 09:00 in Moscow is 06:00 UTC: the second day is still included.
        assert_eq!(rule.starts(dt("2026-01-01T09:00"), &moscow).count(), 2);
    }

    #[test]
    fn rules_that_never_match_end_instead_of_looping() {
        assert_eq!(
            first(
                "FREQ=MONTHLY;BYMONTHDAY=31;BYMONTH=2",
                "2026-01-31T09:00",
                5
            )
            .len(),
            1
        );
    }

    #[test]
    fn parses_and_prints_canonically() {
        let rule: Rrule = "rrule:byday=fr,mo;freq=weekly;interval=1;wkst=MO"
            .parse()
            .unwrap();
        assert_eq!(rule.to_string(), "FREQ=WEEKLY;BYDAY=FR,MO");
        let rule: Rrule = "FREQ=MONTHLY;INTERVAL=3;COUNT=4;BYDAY=-1FR"
            .parse()
            .unwrap();
        assert_eq!(
            rule.to_string(),
            "FREQ=MONTHLY;INTERVAL=3;COUNT=4;BYDAY=-1FR"
        );
        let rule: Rrule = "FREQ=DAILY;UNTIL=20260103T090000Z".parse().unwrap();
        assert_eq!(rule.to_string(), "FREQ=DAILY;UNTIL=20260103T090000Z");
        let rule: Rrule = "FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=-1;WKST=SU;UNTIL=20300101"
            .parse()
            .unwrap();
        assert_eq!(
            rule.to_string(),
            "FREQ=YEARLY;UNTIL=20300101;BYMONTHDAY=-1;BYMONTH=2;WKST=SU"
        );
    }

    #[rstest]
    #[case("", "rrule.invalid")]
    #[case("INTERVAL=2", "rrule.invalid")]
    #[case("FREQ=DAILY;INTERVAL=0", "rrule.invalid")]
    #[case("FREQ=DAILY;COUNT=2;UNTIL=20260101", "rrule.invalid")]
    #[case("FREQ=WEEKLY;BYDAY=XX", "rrule.invalid")]
    #[case("FREQ=WEEKLY;BYDAY=2MO", "rrule.invalid")]
    #[case("FREQ=MONTHLY;BYMONTHDAY=32", "rrule.invalid")]
    #[case("FREQ=HOURLY", "rrule.unsupported")]
    #[case("FREQ=DAILY;BYHOUR=9", "rrule.unsupported")]
    #[case("FREQ=YEARLY;BYDAY=1MO", "rrule.unsupported")]
    fn rejects_bad_rules(#[case] rule: &str, #[case] code: &str) {
        assert_eq!(rule.parse::<Rrule>().unwrap_err().code, code, "{rule}");
    }
}
