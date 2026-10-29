//! Amounts in words for Russian invoices («сумма прописью»): rubles in
//! words, kopecks in digits, as accountants write them.

/// "Одна тысяча двести рублей 05 копеек" for 120 005 kopecks.
pub fn rubles_in_words(minor: i64) -> String {
    let negative = minor < 0;
    let minor = minor.unsigned_abs();
    let (rubles, kopecks) = (minor / 100, minor % 100);
    let mut words = if rubles == 0 {
        "ноль".to_owned()
    } else {
        number(rubles, Gender::Male)
    };
    words.push(' ');
    words.push_str(plural(rubles, ["рубль", "рубля", "рублей"]));
    let mut text = format!(
        "{words} {kopecks:02} {}",
        plural(kopecks, ["копейка", "копейки", "копеек"])
    );
    if negative {
        text = format!("минус {text}");
    }
    let mut chars = text.chars();
    let first = chars.next().expect("never empty");
    first.to_uppercase().chain(chars).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Gender {
    Male,
    Female,
}

/// The form after a number: 1 рубль, 2 рубля, 5 рублей (11–14 too).
fn plural(n: u64, forms: [&'static str; 3]) -> &'static str {
    match (n % 10, n % 100) {
        (_, 11..=14) => forms[2],
        (1, _) => forms[0],
        (2..=4, _) => forms[1],
        _ => forms[2],
    }
}

const UNITS_MALE: [&str; 10] = [
    "",
    "один",
    "два",
    "три",
    "четыре",
    "пять",
    "шесть",
    "семь",
    "восемь",
    "девять",
];
const UNITS_FEMALE: [&str; 3] = ["", "одна", "две"];
const TEENS: [&str; 10] = [
    "десять",
    "одиннадцать",
    "двенадцать",
    "тринадцать",
    "четырнадцать",
    "пятнадцать",
    "шестнадцать",
    "семнадцать",
    "восемнадцать",
    "девятнадцать",
];
const TENS: [&str; 10] = [
    "",
    "",
    "двадцать",
    "тридцать",
    "сорок",
    "пятьдесят",
    "шестьдесят",
    "семьдесят",
    "восемьдесят",
    "девяносто",
];
const HUNDREDS: [&str; 10] = [
    "",
    "сто",
    "двести",
    "триста",
    "четыреста",
    "пятьсот",
    "шестьсот",
    "семьсот",
    "восемьсот",
    "девятьсот",
];

/// Words for 1–999 in the given gender.
fn triple(n: u64, gender: Gender) -> Vec<&'static str> {
    let (h, rest) = ((n / 100) as usize, n % 100);
    let mut out = vec![HUNDREDS[h]];
    if (10..20).contains(&rest) {
        out.push(TEENS[(rest - 10) as usize]);
    } else {
        out.push(TENS[(rest / 10) as usize]);
        let unit = (rest % 10) as usize;
        out.push(match gender {
            Gender::Female if unit <= 2 => UNITS_FEMALE[unit],
            _ => UNITS_MALE[unit],
        });
    }
    out.retain(|w| !w.is_empty());
    out
}

/// Words for a positive number; the last group takes `gender`.
fn number(n: u64, gender: Gender) -> String {
    // (size, gender, forms) from the largest group down.
    let scales: [(u64, Gender, Option<[&str; 3]>); 4] = [
        (
            1_000_000_000,
            Gender::Male,
            Some(["миллиард", "миллиарда", "миллиардов"]),
        ),
        (
            1_000_000,
            Gender::Male,
            Some(["миллион", "миллиона", "миллионов"]),
        ),
        (1_000, Gender::Female, Some(["тысяча", "тысячи", "тысяч"])),
        (1, gender, None),
    ];
    let mut words: Vec<String> = Vec::new();
    let mut rest = n;
    for (size, group_gender, forms) in scales {
        let group = rest / size;
        rest %= size;
        if group == 0 {
            continue;
        }
        // Above a billion the count of billions may itself exceed 999.
        if group >= 1000 {
            words.push(number(group, group_gender));
        } else {
            words.extend(triple(group, group_gender).into_iter().map(str::to_owned));
        }
        if let Some(forms) = forms {
            words.push(plural(group, forms).to_owned());
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0, "Ноль рублей 00 копеек")]
    #[case(100, "Один рубль 00 копеек")]
    #[case(201, "Два рубля 01 копейка")]
    #[case(502, "Пять рублей 02 копейки")]
    #[case(1105, "Одиннадцать рублей 05 копеек")]
    #[case(1211, "Двенадцать рублей 11 копеек")]
    #[case(1421, "Четырнадцать рублей 21 копейка")]
    #[case(2122, "Двадцать один рубль 22 копейки")]
    #[case(2299, "Двадцать два рубля 99 копеек")]
    #[case(11_100, "Сто одиннадцать рублей 00 копеек")]
    #[case(100_000, "Одна тысяча рублей 00 копеек")]
    #[case(200_000, "Две тысячи рублей 00 копеек")]
    #[case(500_000, "Пять тысяч рублей 00 копеек")]
    #[case(1_100_000, "Одиннадцать тысяч рублей 00 копеек")]
    #[case(2_100_000, "Двадцать одна тысяча рублей 00 копеек")]
    #[case(
        12_345_678,
        "Сто двадцать три тысячи четыреста пятьдесят шесть рублей 78 копеек"
    )]
    #[case(100_000_000, "Один миллион рублей 00 копеек")]
    #[case(250_100_000, "Два миллиона пятьсот одна тысяча рублей 00 копеек")]
    #[case(500_000_100, "Пять миллионов один рубль 00 копеек")]
    #[case(300_000_000_000, "Три миллиарда рублей 00 копеек")]
    #[case(1_000_000_100_000, "Десять миллиардов одна тысяча рублей 00 копеек")]
    fn spells_rubles(#[case] minor: i64, #[case] words: &str) {
        assert_eq!(rubles_in_words(minor), words);
    }

    #[test]
    fn negative_amounts_are_spelled_with_minus() {
        assert_eq!(rubles_in_words(-150), "Минус один рубль 50 копеек");
    }
}
