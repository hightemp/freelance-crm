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
    "us_ein",
    "us_ssn_itin",
    "us_aba",
    "us_zip",
    "uk_utr",
    "uk_company_number",
    "uk_vat",
    "uk_sort_code",
    "uk_account",
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
        "us_ein" => (us_ein(value), "us_ein"),
        "us_ssn_itin" => (us_ssn_itin(value), "us_ssn_itin"),
        "us_aba" => (us_aba(value), "us_aba"),
        "us_zip" => (
            pattern(value, "#####") || pattern(value, "#####-####"),
            "us_zip",
        ),
        "uk_utr" => (digits(value, 10), "uk_utr"),
        "uk_company_number" => (uk_company_number(value), "uk_company_number"),
        "uk_vat" => (uk_vat(value), "uk_vat"),
        "uk_sort_code" => (
            pattern(value, "##-##-##") || digits(value, 6),
            "uk_sort_code",
        ),
        "uk_account" => (digits(value, 8), "uk_account"),
        _ => return Err("unknown_validator"),
    };
    if valid { Ok(()) } else { Err(error) }
}

/// Matches a shape where `#` is a digit and other characters are literal.
fn pattern(value: &str, shape: &str) -> bool {
    value.len() == shape.len()
        && value.bytes().zip(shape.bytes()).all(|(v, s)| match s {
            b'#' => v.is_ascii_digit(),
            _ => v == s,
        })
}

/// `NN-NNNNNNN`, the dash optional; prefix 00 is never issued.
fn us_ein(value: &str) -> bool {
    (pattern(value, "##-#######") || digits(value, 9)) && !value.starts_with("00")
}

/// SSN or ITIN `NNN-NN-NNNN` (dashes optional) without the never-issued
/// parts: area 000 or 666, group 00, serial 0000.
fn us_ssn_itin(value: &str) -> bool {
    let compact: String = value.chars().filter(|c| *c != '-').collect();
    let shape = pattern(value, "###-##-####") || digits(value, 9);
    shape
        && !matches!(&compact[..3], "000" | "666")
        && &compact[3..5] != "00"
        && &compact[5..] != "0000"
}

/// ABA routing number: weights 3, 7, 1 repeating; the sum ends in 0.
fn us_aba(value: &str) -> bool {
    const WEIGHTS: [u32; 3] = [3, 7, 1];
    digits(value, 9)
        && digit_values(value)
            .iter()
            .enumerate()
            .map(|(i, d)| d * WEIGHTS[i % 3])
            .sum::<u32>()
            .is_multiple_of(10)
}

/// Eight digits, or a two-letter prefix (SC, NI, OC, …) and six digits.
fn uk_company_number(value: &str) -> bool {
    let b = value.as_bytes();
    digits(value, 8)
        || (b.len() == 8 && b[..2].iter().all(u8::is_ascii_uppercase) && digits(&value[2..], 6))
}

/// `GB` and 9 digits; weights 8..2 over the first seven digits plus the
/// last two must be divisible by 97 (old scheme) or be after adding 55.
fn uk_vat(value: &str) -> bool {
    let Some(number) = value.strip_prefix("GB").filter(|n| digits(n, 9)) else {
        return false;
    };
    let d = digit_values(number);
    let weighted: u32 = d[..7]
        .iter()
        .zip([8, 7, 6, 5, 4, 3, 2])
        .map(|(a, b)| a * b)
        .sum();
    let total = weighted + d[7] * 10 + d[8];
    total.is_multiple_of(97) || (total + 55).is_multiple_of(97)
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
    #[case("us_ein", "12-3456789", true)]
    #[case("us_ein", "123456789", true)]
    #[case("us_ein", "123-456789", false)]
    #[case("us_ein", "12-345678", false)]
    #[case("us_ein", "00-3456789", false)]
    #[case("us_ein", "1A-3456789", false)]
    #[case("us_ssn_itin", "123-45-6789", true)]
    #[case("us_ssn_itin", "923-70-1234", true)]
    #[case("us_ssn_itin", "000-45-6789", false)]
    #[case("us_ssn_itin", "666-45-6789", false)]
    #[case("us_ssn_itin", "123-00-6789", false)]
    #[case("us_ssn_itin", "123-45-0000", false)]
    #[case("us_ssn_itin", "12-345-6789", false)]
    #[case("us_aba", "021000021", true)]
    #[case("us_aba", "011000015", true)]
    #[case("us_aba", "121000358", true)]
    #[case("us_aba", "021000022", false)]
    #[case("us_aba", "02100002", false)]
    #[case("us_zip", "94103", true)]
    #[case("us_zip", "94103-1234", true)]
    #[case("us_zip", "9410", false)]
    #[case("us_zip", "94103-12", false)]
    #[case("uk_utr", "1234567890", true)]
    #[case("uk_utr", "123456789", false)]
    #[case("uk_utr", "12345678901", false)]
    #[case("uk_company_number", "01234567", true)]
    #[case("uk_company_number", "SC123456", true)]
    #[case("uk_company_number", "1234567", false)]
    #[case("uk_company_number", "S1234567", false)]
    #[case("uk_vat", "GB980780684", true)]
    #[case("uk_vat", "GB123456782", true)]
    #[case("uk_vat", "GB434031494", true)]
    #[case("uk_vat", "GB980780685", false)]
    #[case("uk_vat", "980780684", false)]
    #[case("uk_vat", "GB98078068", false)]
    #[case("uk_sort_code", "12-34-56", true)]
    #[case("uk_sort_code", "123456", true)]
    #[case("uk_sort_code", "12-3456", false)]
    #[case("uk_sort_code", "12-34-5", false)]
    #[case("uk_account", "12345678", true)]
    #[case("uk_account", "1234567", false)]
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
