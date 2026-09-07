//! The proleptic Gregorian calendar.

use core::fmt;
use core::str::FromStr;

use crate::error::{Error, Result};
use crate::rata_die::RataDie;
use crate::weekday::Weekday;

/// R.D. of 1 January 1 CE (Gregorian) — the definition of the R.D. epoch.
pub const GREGORIAN_EPOCH: i64 = 1;

/// A date in the proleptic Gregorian calendar.
///
/// "Proleptic" means the Gregorian rules are extended backwards before the 1582
/// reform, and forwards without bound. Year 0 exists and is 1 BCE
/// (astronomical numbering), unlike [`crate::JulianDate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GregorianDate {
    year: i32,
    month: u8,
    day: u8,
}

impl GregorianDate {
    /// Builds a date, validating the month and the day-of-month against the
    /// length of that month in that year.
    pub const fn from_ymd(year: i32, month: u8, day: u8) -> Result<Self> {
        if month < 1 || month > 12 {
            return Err(Error::MonthOutOfRange {
                month: month as i64,
            });
        }
        let max = days_in_month(year, month);
        if day < 1 || day > max {
            return Err(Error::DayOutOfRange {
                day: day as i64,
                max,
            });
        }
        Ok(Self { year, month, day })
    }

    /// The year (astronomical numbering: `0` is 1 BCE, `-1` is 2 BCE).
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month, `1..=12`.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, `1..=31`.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// `(year, month, day)`.
    #[must_use]
    pub const fn as_tuple(self) -> (i32, u8, u8) {
        (self.year, self.month, self.day)
    }

    /// Whether this is a Gregorian leap year.
    #[must_use]
    pub const fn is_leap_year(self) -> bool {
        is_leap_year(self.year)
    }

    /// Number of days in this date's month.
    #[must_use]
    pub const fn days_in_month(self) -> u8 {
        days_in_month(self.year, self.month)
    }

    /// Converts to a fixed day number.
    ///
    /// Reingold & Dershowitz, *Calendrical Calculations*, eq. (2.17).
    #[must_use]
    pub const fn to_rata_die(self) -> RataDie {
        let y = self.year as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        let prior = y - 1;
        let correction = if m <= 2 {
            0
        } else if is_leap_year(self.year) {
            -1
        } else {
            -2
        };
        RataDie::new(
            GREGORIAN_EPOCH - 1 + 365 * prior + prior.div_euclid(4) - prior.div_euclid(100)
                + prior.div_euclid(400)
                + (367 * m - 362).div_euclid(12)
                + correction
                + d,
        )
    }

    /// Converts from a fixed day number.
    ///
    /// Reingold & Dershowitz, eq. (2.23)–(2.25).
    pub fn from_rata_die(rd: RataDie) -> Result<Self> {
        let date = rd.check_range()?.get();
        let year = gregorian_year_from_rata_die(date);
        if year < i64::from(i32::MIN) || year > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year });
        }
        let year = year as i32;
        let jan1 = Self::new_year(year);
        let prior_days = date - jan1;
        let correction = if date < Self::march1(year) {
            0
        } else if is_leap_year(year) {
            1
        } else {
            2
        };
        let month = ((12 * (prior_days + correction) + 373).div_euclid(367)) as u8;
        let day = (date - Self::first_of_month(year, month) + 1) as u8;
        Ok(Self { year, month, day })
    }

    const fn new_year(year: i32) -> i64 {
        Self::first_of_month(year, 1)
    }

    const fn march1(year: i32) -> i64 {
        Self::first_of_month(year, 3)
    }

    const fn first_of_month(year: i32, month: u8) -> i64 {
        let y = year as i64;
        let m = month as i64;
        let prior = y - 1;
        let correction = if m <= 2 {
            0
        } else if is_leap_year(year) {
            -1
        } else {
            -2
        };
        GREGORIAN_EPOCH - 1 + 365 * prior + prior.div_euclid(4) - prior.div_euclid(100)
            + prior.div_euclid(400)
            + (367 * m - 362).div_euclid(12)
            + correction
            + 1
    }

    /// Converts to an integer Julian Day Number (noon-based).
    #[must_use]
    pub const fn to_jdn(self) -> i64 {
        self.to_rata_die().to_jdn()
    }

    /// Converts from an integer Julian Day Number.
    pub fn from_jdn(jdn: i64) -> Result<Self> {
        Self::from_rata_die(RataDie::from_jdn(jdn))
    }

    /// The day of the week.
    #[must_use]
    pub const fn weekday(self) -> Weekday {
        self.to_rata_die().weekday()
    }

    /// Day of the year, `1..=366`.
    #[must_use]
    pub const fn day_of_year(self) -> u16 {
        (self.to_rata_die().get() - Self::first_of_month(self.year, 1) + 1) as u16
    }

    /// Adds a signed number of days.
    pub fn add_days(self, days: i64) -> Result<Self> {
        let rd = self
            .to_rata_die()
            .checked_add_days(days)
            .ok_or(Error::Overflow)?;
        Self::from_rata_die(rd)
    }

    /// Number of days from `other` to `self`.
    #[must_use]
    pub const fn days_since(self, other: Self) -> i64 {
        self.to_rata_die().get() - other.to_rata_die().get()
    }
}

/// Whether `year` is a Gregorian leap year, correct for negative years.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    let y = year as i64;
    y.rem_euclid(4) == 0 && !matches!(y.rem_euclid(400), 100 | 200 | 300)
}

/// Number of days in a Gregorian month.
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn gregorian_year_from_rata_die(date: i64) -> i64 {
    let d0 = date - GREGORIAN_EPOCH;
    let n400 = d0.div_euclid(146_097);
    let d1 = d0.rem_euclid(146_097);
    let n100 = d1.div_euclid(36_524);
    let d2 = d1.rem_euclid(36_524);
    let n4 = d2.div_euclid(1_461);
    let d3 = d2.rem_euclid(1_461);
    let n1 = d3.div_euclid(365);
    let year = 400 * n400 + 100 * n100 + 4 * n4 + n1;
    if n100 == 4 || n1 == 4 {
        year
    } else {
        year + 1
    }
}

impl fmt::Display for GregorianDate {
    /// ISO-8601-like: `2024-09-11`, with a sign for negative years.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.year < 0 {
            write!(
                f,
                "-{:04}-{:02}-{:02}",
                -(self.year as i64),
                self.month,
                self.day
            )
        } else {
            write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
        }
    }
}

impl FromStr for GregorianDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (y, m, d) = crate::parse::parse_ymd(s)?;
        if !(1..=12).contains(&m) {
            return Err(Error::MonthOutOfRange { month: m });
        }
        if y < i64::from(i32::MIN) || y > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year: y });
        }
        let max = days_in_month(y as i32, m as u8);
        if !(1..=i64::from(max)).contains(&d) {
            return Err(Error::DayOutOfRange { day: d, max });
        }
        Self::from_ymd(y as i32, m as u8, d as u8)
    }
}

impl TryFrom<RataDie> for GregorianDate {
    type Error = Error;
    fn try_from(rd: RataDie) -> Result<Self> {
        Self::from_rata_die(rd)
    }
}

impl From<GregorianDate> for RataDie {
    fn from(d: GregorianDate) -> Self {
        d.to_rata_die()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_anchors() {
        assert_eq!(
            GregorianDate::from_ymd(1, 1, 1)
                .unwrap()
                .to_rata_die()
                .get(),
            1
        );
        assert_eq!(
            GregorianDate::from_ymd(2000, 1, 1)
                .unwrap()
                .to_rata_die()
                .get(),
            730_120
        );
        assert_eq!(
            GregorianDate::from_ymd(1970, 1, 1)
                .unwrap()
                .to_rata_die()
                .get(),
            719_163
        );
        // Meeus, Astronomical Algorithms, example 7.a
        assert_eq!(
            GregorianDate::from_ymd(1957, 10, 4).unwrap().to_jdn(),
            2_436_116
        );
    }

    #[test]
    fn leap_year_rule() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        // Proleptic, negative years: year 0 is a leap year (1 BCE).
        assert!(is_leap_year(0));
        assert!(!is_leap_year(-1));
        assert!(is_leap_year(-4));
        assert!(!is_leap_year(-100));
        assert!(is_leap_year(-400));
    }

    #[test]
    fn rejects_impossible_dates() {
        assert!(GregorianDate::from_ymd(2023, 2, 29).is_err());
        assert!(GregorianDate::from_ymd(2024, 2, 29).is_ok());
        assert!(GregorianDate::from_ymd(2024, 13, 1).is_err());
        assert!(GregorianDate::from_ymd(2024, 4, 31).is_err());
    }

    #[test]
    fn roundtrip_dense() {
        let start = GregorianDate::from_ymd(1500, 1, 1)
            .unwrap()
            .to_rata_die()
            .get();
        let end = GregorianDate::from_ymd(2600, 1, 1)
            .unwrap()
            .to_rata_die()
            .get();
        for rd in start..end {
            let d = GregorianDate::from_rata_die(RataDie::new(rd)).unwrap();
            assert_eq!(d.to_rata_die().get(), rd, "roundtrip failed at rd={rd}");
        }
    }

    #[test]
    fn roundtrip_negative_years() {
        let start = GregorianDate::from_ymd(-500, 1, 1)
            .unwrap()
            .to_rata_die()
            .get();
        for rd in start..start + 400_000 {
            let d = GregorianDate::from_rata_die(RataDie::new(rd)).unwrap();
            assert_eq!(d.to_rata_die().get(), rd);
        }
    }

    #[test]
    fn parsing() {
        assert_eq!(
            "2024-09-11".parse::<GregorianDate>().unwrap(),
            GregorianDate::from_ymd(2024, 9, 11).unwrap()
        );
        assert_eq!(
            "-0044-03-15".parse::<GregorianDate>().unwrap().as_tuple(),
            (-44, 3, 15)
        );
        assert!("2023-02-29".parse::<GregorianDate>().is_err());
        assert!("not-a-date".parse::<GregorianDate>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        #[cfg(feature = "alloc")]
        {
            extern crate alloc;
            use alloc::string::ToString;
            let d = GregorianDate::from_ymd(2024, 9, 11).unwrap();
            assert_eq!(d.to_string(), "2024-09-11");
            assert_eq!(d.to_string().parse::<GregorianDate>().unwrap(), d);
        }
    }
}
