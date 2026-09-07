//! Days of the week, with their Amharic and Ge'ez names.

use core::fmt;

/// A day of the week.
///
/// The discriminants match the R.D. convention (`rd mod 7`), i.e.
/// `Sunday == 0`, which is also the convention used by Reingold & Dershowitz
/// and by the Ethiopian liturgical reckoning where the week begins on Sunday
/// (እሑድ, "the first").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Weekday {
    /// እሑድ — Sunday.
    Sunday = 0,
    /// ሰኞ — Monday.
    Monday = 1,
    /// ማክሰኞ — Tuesday.
    Tuesday = 2,
    /// ረቡዕ — Wednesday.
    Wednesday = 3,
    /// ሐሙስ — Thursday.
    Thursday = 4,
    /// ዓርብ — Friday.
    Friday = 5,
    /// ቅዳሜ — Saturday.
    Saturday = 6,
}

/// All seven weekdays, in R.D. order (Sunday first).
pub const ALL_WEEKDAYS: [Weekday; 7] = [
    Weekday::Sunday,
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
];

impl Weekday {
    /// Builds a weekday from a day count where `0` is Sunday, using Euclidean
    /// remainder so negative counts (dates before the R.D. epoch) work.
    #[must_use]
    pub const fn from_days_since_sunday(days: i64) -> Self {
        match days.rem_euclid(7) {
            0 => Self::Sunday,
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            _ => Self::Saturday,
        }
    }

    /// `0` for Sunday through `6` for Saturday.
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// ISO-8601 weekday number: `1` for Monday through `7` for Sunday.
    #[must_use]
    pub const fn iso_number(self) -> u8 {
        match self {
            Self::Sunday => 7,
            other => other as u8,
        }
    }

    /// Amharic name, e.g. `"ሰኞ"`.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::Sunday => "እሑድ",
            Self::Monday => "ሰኞ",
            Self::Tuesday => "ማክሰኞ",
            Self::Wednesday => "ረቡዕ",
            Self::Thursday => "ሐሙስ",
            Self::Friday => "ዓርብ",
            Self::Saturday => "ቅዳሜ",
        }
    }

    /// Ge'ez (liturgical) name, e.g. `"ሰኑይ"` for Monday.
    #[must_use]
    pub const fn geez(self) -> &'static str {
        match self {
            Self::Sunday => "እሑድ",
            Self::Monday => "ሰኑይ",
            Self::Tuesday => "ሠሉስ",
            Self::Wednesday => "ራብዕ",
            Self::Thursday => "ሐሙስ",
            Self::Friday => "ዓርብ",
            Self::Saturday => "ቀዳሚት",
        }
    }

    /// Romanised Amharic name, e.g. `"Segno"`.
    #[must_use]
    pub const fn transliterated(self) -> &'static str {
        match self {
            Self::Sunday => "Ehud",
            Self::Monday => "Segno",
            Self::Tuesday => "Maksegno",
            Self::Wednesday => "Rob",
            Self::Thursday => "Hamus",
            Self::Friday => "Arb",
            Self::Saturday => "Kidame",
        }
    }

    /// English name, e.g. `"Monday"`.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }

    /// Three-letter English abbreviation, e.g. `"Mon"`.
    #[must_use]
    pub const fn english_abbrev(self) -> &'static str {
        match self {
            Self::Sunday => "Sun",
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
        }
    }

    /// The following weekday.
    #[must_use]
    pub const fn succ(self) -> Self {
        Self::from_days_since_sunday(self as i64 + 1)
    }

    /// The preceding weekday.
    #[must_use]
    pub const fn pred(self) -> Self {
        Self::from_days_since_sunday(self as i64 - 1)
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_days_wrap_correctly() {
        assert_eq!(Weekday::from_days_since_sunday(-1), Weekday::Saturday);
        assert_eq!(Weekday::from_days_since_sunday(-7), Weekday::Sunday);
        assert_eq!(Weekday::from_days_since_sunday(-8), Weekday::Saturday);
    }

    #[test]
    fn iso_numbering() {
        assert_eq!(Weekday::Monday.iso_number(), 1);
        assert_eq!(Weekday::Sunday.iso_number(), 7);
        assert_eq!(Weekday::Saturday.iso_number(), 6);
    }

    #[test]
    fn succ_pred_roundtrip() {
        for w in ALL_WEEKDAYS {
            assert_eq!(w.succ().pred(), w);
            assert_eq!(w.pred().succ(), w);
        }
    }
}
