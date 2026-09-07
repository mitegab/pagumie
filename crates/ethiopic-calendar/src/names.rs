//! Names, in Amharic, Ge'ez and Latin script — plus Ge'ez numerals.
//!
//! The tables are `const` arrays indexed by `month - 1` so that name lookup is
//! a bounds-checked index, never a match on a `u8` that silently falls through
//! to a default.

/// Amharic month names, index `month - 1`.
pub const MONTHS_AMHARIC: [&str; 13] = [
    "መስከረም",
    "ጥቅምት",
    "ኅዳር",
    "ታኅሣሥ",
    "ጥር",
    "የካቲት",
    "መጋቢት",
    "ሚያዝያ",
    "ግንቦት",
    "ሰኔ",
    "ሐምሌ",
    "ነሐሴ",
    "ጳጉሜን",
];

/// Scholarly transliteration (Encyclopaedia Aethiopica conventions).
pub const MONTHS_TRANSLITERATED: [&str; 13] = [
    "Mäskäräm",
    "Ṭəqəmt",
    "Ḫədar",
    "Taḫśaś",
    "Ṭərr",
    "Yäkatit",
    "Mägabit",
    "Miyazya",
    "Gənbot",
    "Säne",
    "Ḥamle",
    "Nähase",
    "Ṗagumen",
];

/// Everyday romanisation, the spellings used in Ethiopian English-language press.
pub const MONTHS_LATIN: [&str; 13] = [
    "Meskerem", "Tikimt", "Hidar", "Tahsas", "Tir", "Yekatit", "Megabit", "Miazia", "Ginbot",
    "Sene", "Hamle", "Nehase", "Pagume",
];

/// Three-letter abbreviations, unique across all thirteen months.
pub const MONTHS_ABBREV: [&str; 13] = [
    "Msk", "Tqm", "Hdr", "Ths", "Tir", "Ykt", "Mgb", "Miy", "Gnb", "Sen", "Hml", "Nhs", "Pag",
];

/// Ethiopic digits ፩–፱ (U+1369–U+1371).
pub const GEEZ_UNITS: [&str; 9] = ["፩", "፪", "፫", "፬", "፭", "፮", "፯", "፰", "፱"];

/// Ethiopic tens ፲–፺ (U+1372–U+137A).
pub const GEEZ_TENS: [&str; 9] = ["፲", "፳", "፴", "፵", "፶", "፷", "፸", "፹", "፺"];

/// Ethiopic hundred, ፻ (U+137B).
pub const GEEZ_HUNDRED: &str = "፻";

/// Ethiopic ten-thousand ("myriad"), ፼ (U+137C).
pub const GEEZ_MYRIAD: &str = "፼";

#[cfg(feature = "alloc")]
mod numerals {
    use super::{GEEZ_HUNDRED, GEEZ_MYRIAD, GEEZ_TENS, GEEZ_UNITS};
    use alloc::string::String;

    /// Renders a number in Ge'ez numerals.
    ///
    /// Ge'ez numerals are **not positional** and have **no zero**. Numbers are
    /// grouped by myriads (10⁴), like Chinese numerals, rather than by
    /// thousands:
    ///
    /// ```text
    ///        1  ፩            100  ፻              10 000  ፼
    ///       10  ፲          1 000  ፲፻             20 000  ፪፼
    ///       17  ፲፯         2 017  ፳፻፲፯        1 000 000  ፻፼
    /// ```
    ///
    /// A leading ፩ is elided before ፻ and ፼ (100 is ፻, never ፩፻), which is the
    /// detail most implementations miss.
    ///
    /// Zero has no Ge'ez representation and is rendered as ASCII `"0"`;
    /// negative numbers get an ASCII `-` prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use ethiopic_calendar::names::geez_numeral;
    /// assert_eq!(geez_numeral(2017), "፳፻፲፯");
    /// assert_eq!(geez_numeral(100), "፻");
    /// assert_eq!(geez_numeral(10_000), "፼");
    /// ```
    #[must_use]
    pub fn geez_numeral(n: i64) -> String {
        let mut out = String::new();
        if n == 0 {
            out.push('0');
            return out;
        }
        let mut v = n;
        if v < 0 {
            out.push('-');
            v = -v;
        }
        write_geez(&mut out, v as u64);
        out
    }

    fn write_geez(out: &mut String, n: u64) {
        if n == 0 {
            return;
        }
        if n < 100 {
            write_two_digits(out, n as u8);
        } else if n < 10_000 {
            let hundreds = n / 100;
            if hundreds != 1 {
                write_two_digits(out, hundreds as u8);
            }
            out.push_str(GEEZ_HUNDRED);
            write_geez(out, n % 100);
        } else {
            let myriads = n / 10_000;
            if myriads != 1 {
                write_geez(out, myriads);
            }
            out.push_str(GEEZ_MYRIAD);
            write_geez(out, n % 10_000);
        }
    }

    fn write_two_digits(out: &mut String, n: u8) {
        debug_assert!(n < 100);
        let tens = n / 10;
        let units = n % 10;
        if tens > 0 {
            out.push_str(GEEZ_TENS[tens as usize - 1]);
        }
        if units > 0 {
            out.push_str(GEEZ_UNITS[units as usize - 1]);
        }
    }
}

#[cfg(feature = "alloc")]
pub use numerals::geez_numeral;

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn units_and_tens() {
        assert_eq!(geez_numeral(1), "፩");
        assert_eq!(geez_numeral(9), "፱");
        assert_eq!(geez_numeral(10), "፲");
        assert_eq!(geez_numeral(11), "፲፩");
        assert_eq!(geez_numeral(90), "፺");
        assert_eq!(geez_numeral(99), "፺፱");
    }

    #[test]
    fn hundreds_elide_the_leading_one() {
        assert_eq!(geez_numeral(100), "፻");
        assert_eq!(geez_numeral(101), "፻፩");
        assert_eq!(geez_numeral(110), "፻፲");
        assert_eq!(geez_numeral(200), "፪፻");
        assert_eq!(geez_numeral(1_000), "፲፻");
        assert_eq!(geez_numeral(2_017), "፳፻፲፯");
        assert_eq!(geez_numeral(2_024), "፳፻፳፬");
    }

    #[test]
    fn myriad_grouping() {
        assert_eq!(geez_numeral(10_000), "፼");
        assert_eq!(geez_numeral(10_001), "፼፩");
        assert_eq!(geez_numeral(20_000), "፪፼");
        assert_eq!(geez_numeral(1_000_000), "፻፼");
        assert_eq!(geez_numeral(100_000_000), "፼፼");
    }

    #[test]
    fn zero_and_negative() {
        assert_eq!(geez_numeral(0), "0");
        assert_eq!(geez_numeral(-7), "-፯");
    }

    #[test]
    fn every_month_name_is_distinct() {
        for i in 0..13 {
            for j in 0..13 {
                if i != j {
                    assert_ne!(MONTHS_AMHARIC[i], MONTHS_AMHARIC[j]);
                    assert_ne!(MONTHS_LATIN[i], MONTHS_LATIN[j]);
                    assert_ne!(MONTHS_ABBREV[i], MONTHS_ABBREV[j]);
                    assert_ne!(MONTHS_TRANSLITERATED[i], MONTHS_TRANSLITERATED[j]);
                }
            }
        }
        assert_eq!(MONTHS_ABBREV[0].to_string(), "Msk");
    }
}
