//! The proleptic Julian calendar.
//!
//! Included because it is not optional plumbing here: the Ethiopian calendar's
//! epoch is defined by a *Julian* date (29 August 8 CE), the Ethiopian leap
//! cycle is the Julian one shifted by a year, and the Baḥire Ḥasab computus is
//! the Alexandrian paschal cycle, which is Julian. Anchoring the Ethiopic epoch
//! on a Gregorian date is a two-day error that silently corrupts every
//! conversion before the modern era.

use core::fmt;
use core::str::FromStr;

use crate::error::{Error, Result};
use crate::rata_die::RataDie;
use crate::weekday::Weekday;

/// R.D. of 1 January 1 CE in the Julian calendar (two days before the
/// Gregorian one).
pub const JULIAN_EPOCH: i64 = -1;

/// A date in the proleptic Julian calendar.
///
/// There is **no year 0**: the year before 1 CE is `-1` (1 BCE). Constructing
/// year 0 returns [`Error::JulianYearZero`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JulianDate {
    year: i32,
    month: u8,
    day: u8,
}

impl JulianDate {
    /// Builds a Julian date, rejecting year 0 and impossible day-of-month values.
    pub const fn from_ymd(year: i32, month: u8, day: u8) -> Result<Self> {
        if year == 0 {
            return Err(Error::JulianYearZero);
        }
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

    /// The year; negative values are BCE, and `0` never occurs.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month, `1..=12`.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// `(year, month, day)`.
    #[must_use]
    pub const fn as_tuple(self) -> (i32, u8, u8) {
        (self.year, self.month, self.day)
    }

    /// Whether this is a Julian leap year (every fourth year, no exceptions).
    #[must_use]
    pub const fn is_leap_year(self) -> bool {
        is_leap_year(self.year)
    }

    /// Converts to a fixed day number (Reingold & Dershowitz, eq. 3.3).
    #[must_use]
    pub const fn to_rata_die(self) -> RataDie {
        RataDie::new(first_of_month_rd(self.year, self.month) + self.day as i64 - 1)
    }

    /// Converts from a fixed day number (Reingold & Dershowitz, eq. 3.4).
    pub fn from_rata_die(rd: RataDie) -> Result<Self> {
        let date = rd.check_range()?.get();
        let approx = (4 * (date - JULIAN_EPOCH) + 1464).div_euclid(1_461);
        let year = if approx <= 0 { approx - 1 } else { approx };
        if year < i64::from(i32::MIN) || year > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year });
        }
        let year = year as i32;
        let prior_days = date - first_of_month_rd(year, 1);
        let correction = if date < first_of_month_rd(year, 3) {
            0
        } else if is_leap_year(year) {
            1
        } else {
            2
        };
        let month = ((12 * (prior_days + correction) + 373).div_euclid(367)) as u8;
        let day = (date - first_of_month_rd(year, month) + 1) as u8;
        Ok(Self { year, month, day })
    }

    /// Converts to an integer Julian Day Number.
    ///
    /// Note the naming trap: the *Julian Day Number* has nothing to do with the
    /// *Julian calendar*; it is a continuous day count introduced by Scaliger.
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

    /// Adds a signed number of days.
    pub fn add_days(self, days: i64) -> Result<Self> {
        let rd = self
            .to_rata_die()
            .checked_add_days(days)
            .ok_or(Error::Overflow)?;
        Self::from_rata_die(rd)
    }
}

/// Whether `year` is a Julian leap year.
///
/// Uses the astronomical convention internally: with no year 0, BCE years
/// are leap when `year mod 4 == 3` counting `-1` as 1 BCE.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    let y = year as i64;
    if y > 0 {
        y.rem_euclid(4) == 0
    } else {
        y.rem_euclid(4) == 3
    }
}

/// Number of days in a Julian month.
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

const fn first_of_month_rd(year: i32, month: u8) -> i64 {
    let y = year as i64;
    // Shift BCE years up by one so the arithmetic sees a continuous sequence.
    let y2 = if y < 0 { y + 1 } else { y };
    let m = month as i64;
    let correction = if m <= 2 {
        0
    } else if is_leap_year(year) {
        -1
    } else {
        -2
    };
    JULIAN_EPOCH - 1 + 365 * (y2 - 1) + (y2 - 1).div_euclid(4) + (367 * m - 362).div_euclid(12)
        + correction
        + 1
}

impl PartialOrd for JulianDate {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JulianDate {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Cannot derive: year -1 precedes year 1 but there is no year 0, and
        // lexicographic ordering on the raw fields happens to be right — still,
        // ordering by fixed day number is the definition we want to guarantee.
        self.to_rata_die().cmp(&other.to_rata_die())
    }
}

impl fmt::Display for JulianDate {
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

impl FromStr for JulianDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (y, m, d) = crate::parse::parse_ymd(s)?;
        if y == 0 {
            return Err(Error::JulianYearZero);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gregorian::GregorianDate;

    #[test]
    fn epoch_and_offsets() {
        assert_eq!(JulianDate::from_ymd(1, 1, 1).unwrap().to_rata_die().get(), -1);
        // The Julian calendar was 13 days behind Gregorian from 1900 to 2100.
        let j = JulianDate::from_ymd(2024, 8, 29).unwrap();
        assert_eq!(
            GregorianDate::from_rata_die(j.to_rata_die()).unwrap().as_tuple(),
            (2024, 9, 11)
        );
        // 10 days at the 1582 reform: 1582-10-04 Julian is followed by
        // 1582-10-15 Gregorian.
        let j = JulianDate::from_ymd(1582, 10, 4).unwrap();
        assert_eq!(
            GregorianDate::from_rata_die(j.to_rata_die() + 1).unwrap().as_tuple(),
            (1582, 10, 15)
        );
    }

    #[test]
    fn no_year_zero() {
        assert_eq!(JulianDate::from_ymd(0, 1, 1), Err(Error::JulianYearZero));
        assert_eq!("0000-01-01".parse::<JulianDate>(), Err(Error::JulianYearZero));
        // -1 is 1 BCE and immediately precedes 1 CE.
        let bce1 = JulianDate::from_ymd(-1, 12, 31).unwrap();
        let ce1 = JulianDate::from_ymd(1, 1, 1).unwrap();
        assert_eq!(ce1.to_rata_die().get() - bce1.to_rata_die().get(), 1);
        assert!(bce1 < ce1);
    }

    #[test]
    fn bce_leap_years() {
        // 1 BCE, 5 BCE, ... are leap years in the proleptic Julian calendar.
        assert!(is_leap_year(-1));
        assert!(is_leap_year(-5));
        assert!(!is_leap_year(-2));
        assert!(JulianDate::from_ymd(-1, 2, 29).is_ok());
        assert!(JulianDate::from_ymd(-2, 2, 29).is_err());
    }

    #[test]
    fn roundtrip_dense_including_bce() {
        let start = JulianDate::from_ymd(-200, 1, 1).unwrap().to_rata_die().get();
        let end = JulianDate::from_ymd(2200, 1, 1).unwrap().to_rata_die().get();
        for rd in start..end {
            let d = JulianDate::from_rata_die(RataDie::new(rd)).unwrap();
            assert_eq!(d.to_rata_die().get(), rd, "roundtrip failed at rd={rd}");
            assert_ne!(d.year(), 0, "year 0 produced at rd={rd}");
        }
    }
}
