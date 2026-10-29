//! Money is stored as an integer amount of minor units (e.g. kopecks, cents)
//! paired with an ISO 4217 currency. Floats are never used.

use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("unknown currency code")]
    UnknownCurrency,
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(Currency, Currency),
    #[error("amount overflow")]
    Overflow,
}

/// ISO 4217 currency with the number of digits after the decimal point.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(try_from = "String", into = "String")]
#[specta(type = String)]
pub struct Currency {
    code: [u8; 3],
    minor_units: u8,
}

/// Active ISO 4217 codes (fund and precious-metal codes excluded).
const ISO_4217: &str = "AED AFN ALL AMD ANG AOA ARS AUD AWG AZN BAM BBD BDT BGN BHD BIF BMD BND \
BOB BRL BSD BTN BWP BYN BZD CAD CDF CHF CLF CLP CNY COP CRC CUP CVE CZK DJF DKK DOP DZD EGP ERN \
ETB EUR FJD FKP GBP GEL GHS GIP GMD GNF GTQ GYD HKD HNL HTG HUF IDR ILS INR IQD IRR ISK JMD JOD \
JPY KES KGS KHR KMF KPW KRW KWD KYD KZT LAK LBP LKR LRD LSL LYD MAD MDL MGA MKD MMK MNT MOP MRU \
MUR MVR MWK MXN MYR MZN NAD NGN NIO NOK NPR NZD OMR PAB PEN PGK PHP PKR PLN PYG QAR RON RSD RUB \
RWF SAR SBD SCR SDG SEK SGD SHP SLE SOS SRD SSP STN SVC SYP SZL THB TJS TMT TND TOP TRY TTD TWD \
TZS UAH UGX USD UYU UYW UZS VED VES VND VUV WST XAF XCD XCG XOF XPF YER ZAR ZMW ZWG";

/// Currencies whose minor unit is not 2 digits.
const NON_DEFAULT_MINOR_UNITS: &[(&str, u8)] = &[
    ("BIF", 0),
    ("CLP", 0),
    ("DJF", 0),
    ("GNF", 0),
    ("ISK", 0),
    ("JPY", 0),
    ("KMF", 0),
    ("KRW", 0),
    ("PYG", 0),
    ("RWF", 0),
    ("UGX", 0),
    ("VND", 0),
    ("VUV", 0),
    ("XAF", 0),
    ("XOF", 0),
    ("XPF", 0),
    ("BHD", 3),
    ("IQD", 3),
    ("JOD", 3),
    ("KWD", 3),
    ("LYD", 3),
    ("OMR", 3),
    ("TND", 3),
    ("CLF", 4),
    ("UYW", 4),
];

impl Currency {
    pub fn code(&self) -> &str {
        // Only ASCII codes from ISO_4217 are ever constructed.
        std::str::from_utf8(&self.code).expect("currency code is ASCII")
    }

    pub fn minor_units(&self) -> u8 {
        self.minor_units
    }
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let code: [u8; 3] = s
            .as_bytes()
            .try_into()
            .map_err(|_| MoneyError::UnknownCurrency)?;
        if !ISO_4217.split_whitespace().any(|c| c == s) {
            return Err(MoneyError::UnknownCurrency);
        }
        let minor_units = NON_DEFAULT_MINOR_UNITS
            .iter()
            .find(|(c, _)| *c == s)
            .map_or(2, |(_, units)| *units);
        Ok(Self { code, minor_units })
    }
}

impl TryFrom<String> for Currency {
    type Error = MoneyError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Currency> for String {
    fn from(c: Currency) -> Self {
        c.code().to_owned()
    }
}

impl ToSql for Currency {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.code()))
    }
}

impl FromSql for Currency {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e: MoneyError| FromSqlError::Other(Box::new(e)))
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    // Exported as a JS number: amounts stay far below 2^53 minor units.
    // `i32` is only the TypeScript type marker, see `Timestamp`.
    #[specta(type = i32)]
    pub amount_minor: i64,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount_minor: i64, currency: Currency) -> Self {
        Self {
            amount_minor,
            currency,
        }
    }

    pub fn zero(currency: Currency) -> Self {
        Self::new(0, currency)
    }

    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        self.same_currency(other)?;
        let amount = self
            .amount_minor
            .checked_add(other.amount_minor)
            .ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    pub fn checked_sub(self, other: Money) -> Result<Money, MoneyError> {
        self.same_currency(other)?;
        let amount = self
            .amount_minor
            .checked_sub(other.amount_minor)
            .ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    /// Amount for `minutes` of work at `self` per hour, rounded half-even
    /// to the nearest minor unit.
    pub fn for_minutes(self, minutes: i64) -> Result<Money, MoneyError> {
        self.scaled(minutes, 60)
    }

    /// `self * basis_points / 10_000`, rounded half-even. 1 bp = 0.01 %.
    pub fn percent_bp(self, basis_points: i64) -> Result<Money, MoneyError> {
        self.scaled(basis_points, 10_000)
    }

    /// `self * num / den` with half-even rounding, computed in i128.
    fn scaled(self, num: i64, den: i64) -> Result<Money, MoneyError> {
        let exact = i128::from(self.amount_minor) * i128::from(num);
        let amount = i64::try_from(div_round_half_even(exact, i128::from(den)))
            .map_err(|_| MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    fn same_currency(self, other: Money) -> Result<(), MoneyError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch(self.currency, other.currency))
        }
    }
}

/// Integer division rounding to the nearest value, ties to even.
fn div_round_half_even(num: i128, den: i128) -> i128 {
    debug_assert!(den > 0);
    let quotient = num.div_euclid(den);
    let remainder = num.rem_euclid(den);
    match (remainder * 2).cmp(&den) {
        std::cmp::Ordering::Less => quotient,
        std::cmp::Ordering::Greater => quotient + 1,
        std::cmp::Ordering::Equal if quotient % 2 == 0 => quotient,
        std::cmp::Ordering::Equal => quotient + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    fn cur(code: &str) -> Currency {
        code.parse().unwrap()
    }

    fn m(amount: i64, code: &str) -> Money {
        Money::new(amount, cur(code))
    }

    #[rstest]
    #[case("RUB", 2)]
    #[case("USD", 2)]
    #[case("EUR", 2)]
    #[case("GBP", 2)]
    #[case("MYR", 2)]
    #[case("JPY", 0)]
    #[case("KWD", 3)]
    fn known_currencies_have_minor_units(#[case] code: &str, #[case] units: u8) {
        let c = cur(code);
        assert_eq!(c.code(), code);
        assert_eq!(c.minor_units(), units);
    }

    #[rstest]
    #[case("rub")]
    #[case("XYZ")]
    #[case("")]
    #[case("RUBL")]
    #[case("РУБ")]
    fn invalid_currency_codes_are_rejected(#[case] code: &str) {
        assert_eq!(code.parse::<Currency>(), Err(MoneyError::UnknownCurrency));
    }

    #[test]
    fn currency_serializes_as_code() {
        assert_eq!(serde_json::to_string(&cur("EUR")).unwrap(), "\"EUR\"");
        let c: Currency = serde_json::from_str("\"JPY\"").unwrap();
        assert_eq!(c, cur("JPY"));
        assert!(serde_json::from_str::<Currency>("\"NOPE\"").is_err());
    }

    #[test]
    fn add_and_sub_same_currency() {
        assert_eq!(m(150, "RUB").checked_add(m(250, "RUB")), Ok(m(400, "RUB")));
        assert_eq!(m(150, "RUB").checked_sub(m(250, "RUB")), Ok(m(-100, "RUB")));
    }

    #[test]
    fn mixing_currencies_is_an_error() {
        assert_eq!(
            m(1, "RUB").checked_add(m(1, "USD")),
            Err(MoneyError::CurrencyMismatch(cur("RUB"), cur("USD")))
        );
        assert!(m(1, "RUB").checked_sub(m(1, "USD")).is_err());
    }

    #[test]
    fn overflow_is_an_error() {
        assert_eq!(
            m(i64::MAX, "USD").checked_add(m(1, "USD")),
            Err(MoneyError::Overflow)
        );
        assert_eq!(
            m(i64::MAX, "USD").for_minutes(120),
            Err(MoneyError::Overflow)
        );
    }

    #[rstest]
    // 1500.00 / h * 90 min = 2250.00
    #[case(150_000, 90, 225_000)]
    // 10.00 / h * 1 min = 0.1666.. -> 0.17
    #[case(1_000, 1, 17)]
    // 0.30 / h * 5 min = 2.5 minor -> ties to even: 2
    #[case(30, 5, 2)]
    // 0.42 / h * 5 min = 3.5 minor -> ties to even: 4
    #[case(42, 5, 4)]
    #[case(1_000, 0, 0)]
    // negative amounts round symmetrically
    #[case(-30, 5, -2)]
    fn amount_for_minutes(#[case] rate: i64, #[case] minutes: i64, #[case] expected: i64) {
        assert_eq!(m(rate, "RUB").for_minutes(minutes), Ok(m(expected, "RUB")));
    }

    #[rstest]
    // 6 % of 1000.00 = 60.00
    #[case(100_000, 600, 6_000)]
    // 4 % of 0.13 = 0.0052 -> 0.01
    #[case(13, 400, 1)]
    // 50 % of 0.05 = 2.5 minor -> 2 (ties to even)
    #[case(5, 5_000, 2)]
    // 50 % of 0.07 = 3.5 minor -> 4
    #[case(7, 5_000, 4)]
    #[case(100, 0, 0)]
    fn percent_in_basis_points(#[case] amount: i64, #[case] bp: i64, #[case] expected: i64) {
        assert_eq!(m(amount, "RUB").percent_bp(bp), Ok(m(expected, "RUB")));
    }

    #[rstest]
    #[case(5, 2, 2)]
    #[case(7, 2, 4)]
    #[case(-5, 2, -2)]
    #[case(-7, 2, -4)]
    #[case(10, 3, 3)]
    #[case(11, 3, 4)]
    #[case(-10, 3, -3)]
    #[case(0, 7, 0)]
    fn half_even_division(#[case] num: i128, #[case] den: i128, #[case] expected: i128) {
        assert_eq!(div_round_half_even(num, den), expected);
    }

    proptest! {
        #[test]
        fn addition_is_commutative_and_associative(
            a in -1_000_000_000_000i64..1_000_000_000_000,
            b in -1_000_000_000_000i64..1_000_000_000_000,
            c in -1_000_000_000_000i64..1_000_000_000_000,
        ) {
            let (a, b, c) = (m(a, "USD"), m(b, "USD"), m(c, "USD"));
            prop_assert_eq!(a.checked_add(b), b.checked_add(a));
            prop_assert_eq!(
                a.checked_add(b).and_then(|ab| ab.checked_add(c)),
                b.checked_add(c).and_then(|bc| a.checked_add(bc)),
            );
        }

        #[test]
        fn rounding_error_is_at_most_half_a_unit(num in -1_000_000_000i128..1_000_000_000, den in 1i128..100_000) {
            let q = div_round_half_even(num, den);
            // |q * den - num| <= den / 2
            prop_assert!((q * den - num).abs() * 2 <= den);
        }
    }

    #[test]
    fn currency_sqlite_round_trip() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let rub: Currency = "RUB".parse().unwrap();
        let back: Currency = conn.query_row("SELECT ?1", [rub], |r| r.get(0)).unwrap();
        assert_eq!(back, rub);
        let bad: rusqlite::Result<Currency> = conn.query_row("SELECT 'ZZZ'", [], |r| r.get(0));
        assert!(bad.is_err());
    }
}
