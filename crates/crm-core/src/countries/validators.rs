//! Named validators referenced from country schemas. A schema may only use
//! names listed in [`NAMES`]; the error is the validator name, which the UI
//! translates as `error_validation_<name>`.

use std::collections::BTreeMap;

/// Values of all requisite fields, for validators that check one field
/// against another (an account against its bank code).
pub type Values = BTreeMap<String, String>;

pub const NAMES: &[&str] = &[
    "ru_inn",
    "ru_inn_10",
    "ru_inn_12",
    "ru_kpp",
    "ru_ogrn",
    "ru_ogrnip",
    "ru_bik",
    "ru_account",
    "ru_corr_account",
    "iban",
    "swift_bic",
];

/// Checks a normalized, non-empty value. Unknown names never pass.
pub fn check(name: &str, value: &str, values: &Values) -> Result<(), &'static str> {
    let bik = || {
        values
            .get("bik")
            .filter(|b| digits(b, 9))
            .map(String::as_str)
    };
    let (valid, error) = match name {
        "ru_inn" => (inn10(value) || inn12(value), "ru_inn"),
        "ru_inn_10" => (inn10(value), "ru_inn_10"),
        "ru_inn_12" => (inn12(value), "ru_inn_12"),
        "ru_kpp" => (kpp(value), "ru_kpp"),
        "ru_ogrn" => (checked_number(value, 13, 11), "ru_ogrn"),
        "ru_ogrnip" => (checked_number(value, 15, 13), "ru_ogrnip"),
        "ru_bik" => (digits(value, 9), "ru_bik"),
        "ru_account" | "ru_corr_account" => {
            let Some(bik) = bik() else {
                return Err("ru_account_needs_bik");
            };
            // The control key covers part of the BIK followed by the account.
            let prefix = if name == "ru_account" {
                bik[6..9].to_owned()
            } else {
                format!("0{}", &bik[4..6])
            };
            let error = if name == "ru_account" {
                "ru_account"
            } else {
                "ru_corr_account"
            };
            (digits(value, 20) && account_key(&(prefix + value)), error)
        }
        "iban" => (iban(value), "iban"),
        "swift_bic" => (swift_bic(value), "swift_bic"),
        _ => return Err("unknown_validator"),
    };
    if valid { Ok(()) } else { Err(error) }
}

fn digits(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_digit())
}

fn digit_values(value: &str) -> Vec<u32> {
    value.bytes().map(|b| u32::from(b - b'0')).collect()
}

/// Weighted sum mod 11 mod 10, as used by INN check digits.
fn inn_check(d: &[u32], weights: &[u32]) -> u32 {
    d.iter().zip(weights).map(|(a, b)| a * b).sum::<u32>() % 11 % 10
}

fn inn10(value: &str) -> bool {
    if !digits(value, 10) {
        return false;
    }
    let d = digit_values(value);
    inn_check(&d, &[2, 4, 10, 3, 5, 9, 4, 6, 8]) == d[9]
}

fn inn12(value: &str) -> bool {
    if !digits(value, 12) {
        return false;
    }
    let d = digit_values(value);
    inn_check(&d, &[7, 2, 4, 10, 3, 5, 9, 4, 6, 8]) == d[10]
        && inn_check(&d, &[3, 7, 2, 4, 10, 3, 5, 9, 4, 6, 8]) == d[11]
}

/// OGRN/OGRNIP: the number without its last digit, mod `modulus`, mod 10,
/// equals the last digit.
fn checked_number(value: &str, len: usize, modulus: u64) -> bool {
    if !digits(value, len) {
        return false;
    }
    let (body, last) = value.split_at(len - 1);
    let body: u64 = body.parse().expect("digits");
    u64::from(last.as_bytes()[0] - b'0') == body % modulus % 10
}

/// Four digits, two digits or capital Latin letters, three digits.
fn kpp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 9
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4..6]
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        && b[6..].iter().all(u8::is_ascii_digit)
}

/// Bank of Russia control key: weights 7, 1, 3 repeating; the sum must end in 0.
fn account_key(control: &str) -> bool {
    const WEIGHTS: [u32; 3] = [7, 1, 3];
    let sum: u32 = digit_values(control)
        .iter()
        .enumerate()
        .map(|(i, d)| d * WEIGHTS[i % 3])
        .sum();
    sum.is_multiple_of(10)
}

/// ISO 13616: country code, check digits, then mod 97 == 1 over the
/// rearranged number with letters as 10..35.
fn iban(value: &str) -> bool {
    let b = value.as_bytes();
    let shape = (15..=34).contains(&b.len())
        && b[..2].iter().all(u8::is_ascii_uppercase)
        && b[2..4].iter().all(u8::is_ascii_digit)
        && b[4..].iter().all(u8::is_ascii_alphanumeric);
    if !shape {
        return false;
    }
    let rearranged = value[4..].chars().chain(value[..4].chars());
    let remainder = rearranged.fold(0u32, |acc, c| {
        let n = c.to_digit(36).expect("alphanumeric");
        let acc = if n >= 10 { acc * 100 + n } else { acc * 10 + n };
        acc % 97
    });
    remainder == 1
}

/// ISO 9362: bank (4 letters), country (2 letters), location (2), optional branch (3).
fn swift_bic(value: &str) -> bool {
    let b = value.as_bytes();
    (b.len() == 8 || b.len() == 11)
        && b[..6].iter().all(u8::is_ascii_uppercase)
        && b[6..].iter().all(u8::is_ascii_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn ok(name: &str, value: &str) -> bool {
        check(name, value, &Values::new()).is_ok()
    }

    fn with_bik(name: &str, value: &str, bik: &str) -> Result<(), &'static str> {
        let values = Values::from([("bik".to_owned(), bik.to_owned())]);
        check(name, value, &values)
    }

    #[rstest]
    #[case("ru_inn", "7707083893", true)]
    #[case("ru_inn", "500100732259", true)]
    #[case("ru_inn", "7707083894", false)]
    #[case("ru_inn", "500100732258", false)]
    #[case("ru_inn", "500100732249", false)]
    #[case("ru_inn", "77070838931", false)]
    #[case("ru_inn", "77070838a3", false)]
    #[case("ru_inn_10", "7707083893", true)]
    #[case("ru_inn_10", "500100732259", false)]
    #[case("ru_inn_12", "500100732259", true)]
    #[case("ru_inn_12", "7707083893", false)]
    #[case("ru_ogrn", "1027700132195", true)]
    #[case("ru_ogrn", "1027700132196", false)]
    #[case("ru_ogrn", "102770013219", false)]
    #[case("ru_ogrnip", "304500116000157", true)]
    #[case("ru_ogrnip", "304500116000158", false)]
    #[case("ru_ogrnip", "1027700132195", false)]
    #[case("ru_kpp", "773601001", true)]
    #[case("ru_kpp", "7736AB001", true)]
    #[case("ru_kpp", "77360100", false)]
    #[case("ru_kpp", "7736010011", false)]
    #[case("ru_kpp", "77a601001", false)]
    #[case("ru_bik", "044525225", true)]
    #[case("ru_bik", "04452522", false)]
    #[case("ru_bik", "04452522x", false)]
    #[case("iban", "GB82WEST12345698765432", true)]
    #[case("iban", "DE89370400440532013000", true)]
    #[case("iban", "GB82WEST12345698765431", false)]
    #[case("iban", "GB82", false)]
    #[case("iban", "1282WEST12345698765432", false)]
    #[case("iban", "GB82WEST1234569876543!", false)]
    #[case("swift_bic", "DEUTDEFF", true)]
    #[case("swift_bic", "DEUTDEFF500", true)]
    #[case("swift_bic", "SABRRUMM", true)]
    #[case("swift_bic", "DEUTDEF", false)]
    #[case("swift_bic", "DEUTDEFF5", false)]
    #[case("swift_bic", "1EUTDEFF", false)]
    #[case("swift_bic", "DEUT1EFF", false)]
    fn single_field_validators(#[case] name: &str, #[case] value: &str, #[case] valid: bool) {
        assert_eq!(ok(name, value), valid, "{name}({value})");
    }

    #[test]
    fn failures_name_the_validator() {
        assert_eq!(check("ru_inn_10", "1", &Values::new()), Err("ru_inn_10"));
    }

    #[test]
    fn unknown_validator_never_passes() {
        assert_eq!(check("nope", "1", &Values::new()), Err("unknown_validator"));
    }

    #[test]
    fn settlement_account_is_checked_against_the_bik() {
        assert_eq!(
            with_bik("ru_account", "40702810938000000001", "044525225"),
            Ok(())
        );
        assert_eq!(
            with_bik("ru_account", "40702810038000000001", "044525225"),
            Err("ru_account")
        );
        assert_eq!(
            with_bik("ru_account", "4070281093800000000", "044525225"),
            Err("ru_account")
        );
        // Without a valid BIK the account cannot be verified.
        assert_eq!(
            check("ru_account", "40702810938000000001", &Values::new()),
            Err("ru_account_needs_bik")
        );
        assert_eq!(
            with_bik("ru_account", "40702810938000000001", "123"),
            Err("ru_account_needs_bik")
        );
    }

    #[test]
    fn correspondent_account_is_checked_against_the_bik() {
        assert_eq!(
            with_bik("ru_corr_account", "30101810400000000225", "044525225"),
            Ok(())
        );
        assert_eq!(
            with_bik("ru_corr_account", "30101810400000000226", "044525225"),
            Err("ru_corr_account")
        );
        assert_eq!(
            check("ru_corr_account", "30101810400000000225", &Values::new()),
            Err("ru_account_needs_bik")
        );
    }
}
