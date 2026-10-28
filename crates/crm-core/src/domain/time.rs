//! UTC timestamps in milliseconds and an injectable clock, so that
//! time-dependent logic (reminders, autolock, overdue) is testable.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// Exported as a JS number: unix milliseconds fit in 2^53.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(transparent)]
pub struct Timestamp(#[specta(type = f64)] pub i64);

impl Timestamp {
    pub fn from_millis(ms: i64) -> Self {
        Self(ms)
    }

    pub fn as_millis(self) -> i64 {
        self.0
    }

    pub fn plus_millis(self, ms: i64) -> Self {
        Self(self.0.saturating_add(ms))
    }
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before 1970")
            .as_millis();
        Timestamp(ms as i64)
    }
}

/// Manually driven clock for tests.
pub struct FakeClock(Mutex<Timestamp>);

impl FakeClock {
    pub fn at(ts: Timestamp) -> Self {
        Self(Mutex::new(ts))
    }

    pub fn advance_millis(&self, ms: i64) {
        let mut now = self.0.lock().unwrap();
        *now = now.plus_millis(ms);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_is_close_to_real_time() {
        let expected = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let now = SystemClock.now().as_millis();
        assert!((now - expected).abs() < 1_000);
    }

    #[test]
    fn fake_clock_advances() {
        let clock = FakeClock::at(Timestamp::from_millis(1_000));
        clock.advance_millis(500);
        assert_eq!(clock.now(), Timestamp::from_millis(1_500));
    }

    #[test]
    fn plus_millis_saturates() {
        assert_eq!(Timestamp(10).plus_millis(5), Timestamp(15));
        assert_eq!(Timestamp(i64::MAX).plus_millis(1), Timestamp(i64::MAX));
    }
}
