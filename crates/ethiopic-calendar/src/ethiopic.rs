//! The proleptic Ethiopian (Ge'ez) calendar — the core of this crate.
//!
//! # Structure of the year
//!
//! Twelve months of exactly thirty days, followed by *Ṗagumēn* (ጳጉሜን), the
//! epagomenal thirteenth month of five days — six in a leap year. The year
//! therefore has 365 or 366 days with no irregular month lengths at all, which
//! makes the arithmetic exact and branch-free.
//!
//! # The leap rule, and why everyone gets it wrong
//!
//! ```text
//! leap(y)  ⟺  y mod 4 == 3          (Ethiopian, Amete Mihret)
//! leap(y)  ⟺  y mod 4 == 0          (Julian — NOT the Ethiopian rule)
//! ```
//!
//! The Ethiopian year begins on 1 Meskerem ≈ 11 September, i.e. *before* the
//! Julian leap day of the following February. So the extra epagomenal day is
//! appended at the end of the Ethiopian year that *precedes* the Julian leap
//! year. Ethiopian 2015 is a leap year; Gregorian 2016 is not, but Julian 2016
//! is, and Ethiopian 2016 is not. Copying the Gregorian/Julian predicate gives
//! the right answer only one year in four.
//!
//! Because the crate uses Euclidean remainder, the rule holds unchanged for
//! years ≤ 0: `(-1) mod 4 == 3`, so year −1 is a leap year, exactly as the
//! backwards extension of the cycle requires.
//!
//! # Epoch
//!
//! | | |
//! |---|---|
//! | 1 Meskerem 1 A.M. | 29 August 8 CE (Julian) |
//! | | 27 August 8 CE (proleptic Gregorian) |
//! | | JDN 1 724 221 |
//! | | R.D. 2796 |
//!
//! (Reingold & Dershowitz, *Calendrical Calculations*, §4.2.)

use core::fmt;
use core::str::FromStr;

use crate::era::{Era, AMETE_ALEM_OFFSET};
use crate::error::{Error, Result};
use crate::gregorian::GregorianDate;
use crate::julian::JulianDate;
use crate::rata_die::RataDie;
use crate::weekday::Weekday;

/// R.D. of 1 Meskerem 1 *Amete Mihret* = 29 August 8 CE (Julian).
pub const ETHIOPIC_EPOCH: i64 = 2796;

/// Julian Day Number of the Ethiopic epoch, 1 Meskerem 1 A.M.
pub const ETHIOPIC_EPOCH_JDN: i64 = ETHIOPIC_EPOCH + crate::rata_die::JDN_OFFSET;

/// The months of the Ethiopian year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Month {
    /// መስከረም — Meskerem (month 1).
    Meskerem = 1,
    /// ጥቅምት — Ṭiqimt (month 2).
    Tikimt = 2,
    /// ኅዳር — Ḫidar (month 3).
    Hidar = 3,
    /// ታኅሣሥ — Taḫśaś (month 4).
    Tahsas = 4,
    /// ጥር — Ṭir (month 5).
    Tir = 5,
    /// የካቲት — Yekatit (month 6).
    Yekatit = 6,
    /// መጋቢት — Megabit (month 7).
    Megabit = 7,
    /// ሚያዝያ — Miyazya (month 8).
    Miazia = 8,
    /// ግንቦት — Ginbot (month 9).
    Ginbot = 9,
    /// ሰኔ — Säne (month 10).
    Sene = 10,
    /// ሐምሌ — Ḥamle (month 11).
    Hamle = 11,
    /// ነሐሴ — Nähase (month 12).
    Nehase = 12,
    /// ጳጉሜን — Ṗagumēn, the epagomenal month of 5 or 6 days (month 13).
    Pagume = 13,
}

/// Every month, in calendar order.
pub const ALL_MONTHS: [Month; 13] = [
    Month::Meskerem,
    Month::Tikimt,
    Month::Hidar,
    Month::Tahsas,
    Month::Tir,
    Month::Yekatit,
    Month::Megabit,
    Month::Miazia,
    Month::Ginbot,
    Month::Sene,
    Month::Hamle,
    Month::Nehase,
    Month::Pagume,
];

impl Month {
    /// Builds a month from its `1..=13` index.
    pub const fn from_number(n: u8) -> Result<Self> {
        Ok(match n {
            1 => Self::Meskerem,
            2 => Self::Tikimt,
            3 => Self::Hidar,
            4 => Self::Tahsas,
            5 => Self::Tir,
            6 => Self::Yekatit,
            7 => Self::Megabit,
            8 => Self::Miazia,
            9 => Self::Ginbot,
            10 => Self::Sene,
            11 => Self::Hamle,
            12 => Self::Nehase,
            13 => Self::Pagume,
            _ => {
                return Err(Error::MonthOutOfRange {
                    month: n as i64,
                })
            }
        })
    }

    /// The `1..=13` index.
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// Whether this is the epagomenal month Ṗagumēn.
    #[must_use]
    pub const fn is_epagomenal(self) -> bool {
        matches!(self, Self::Pagume)
    }

    /// Number of days in this month for the given year.
    #[must_use]
    pub const fn days_in(self, year: i32) -> u8 {
        match self {
            Self::Pagume => {
                if is_leap_year(year) {
                    6
                } else {
                    5
                }
            }
            _ => 30,
        }
    }

    /// Amharic name, e.g. `"መስከረም"`.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        crate::names::MONTHS_AMHARIC[self as usize - 1]
    }

    /// Scholarly transliteration, e.g. `"Mäskäräm"`.
    #[must_use]
    pub const fn transliterated(self) -> &'static str {
        crate::names::MONTHS_TRANSLITERATED[self as usize - 1]
    }

    /// Common romanisation, e.g. `"Meskerem"`.
    #[must_use]
    pub const fn latin(self) -> &'static str {
        crate::names::MONTHS_LATIN[self as usize - 1]
    }

    /// Three-letter Latin abbreviation, e.g. `"Msk"`.
    #[must_use]
    pub const fn abbrev(self) -> &'static str {
        crate::names::MONTHS_ABBREV[self as usize - 1]
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.latin())
    }
}

/// A date in the proleptic Ethiopian calendar, in the *Amete Mihret* era.
///
/// Dates are stored as `(year, month, day)` and are always valid: the
/// constructor rejects `Ṗagumēn 6` in a common year, which is the single most
/// common way to produce a corrupt Ethiopian date.
///
/// Ordering is chronological (derived lexicographic ordering coincides with it
/// because every month has a fixed position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EthiopianDate {
    year: i32,
    month: u8,
    day: u8,
}

impl EthiopianDate {
    /// Builds a date from *Amete Mihret* year, month `1..=13` and day.
    ///
    /// ```
    /// use ethiopic_calendar::EthiopianDate;
    /// assert!(EthiopianDate::from_ymd(2015, 13, 6).is_ok());  // leap year
    /// assert!(EthiopianDate::from_ymd(2016, 13, 6).is_err()); // common year
    /// ```
    pub const fn from_ymd(year: i32, month: u8, day: u8) -> Result<Self> {
        if month < 1 || month > 13 {
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

    /// Builds a date from a year expressed in an explicit [`Era`].
    ///
    /// ```
    /// use ethiopic_calendar::{EthiopianDate, Era};
    /// let a = EthiopianDate::from_ymd_era(7517, 1, 1, Era::AmeteAlem).unwrap();
    /// let b = EthiopianDate::from_ymd(2017, 1, 1).unwrap();
    /// assert_eq!(a, b);
    /// ```
    pub fn from_ymd_era(year: i32, month: u8, day: u8, era: Era) -> Result<Self> {
        let am = era.to_amete_mihret(year)?;
        Self::from_ymd(am, month, day)
    }

    /// The *Amete Mihret* year.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month index, `1..=13`.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The month as an enum.
    #[must_use]
    pub const fn month_enum(self) -> Month {
        match Month::from_number(self.month) {
            Ok(m) => m,
            // Unreachable: the constructor validated the range.
            Err(_) => Month::Meskerem,
        }
    }

    /// The day of the month.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// `(year, month, day)` in the Amete Mihret era.
    #[must_use]
    pub const fn as_tuple(self) -> (i32, u8, u8) {
        (self.year, self.month, self.day)
    }

    /// The year counted in the *Amete Alem* ("year of the world") era,
    /// i.e. `year + 5500`.
    #[must_use]
    pub const fn amete_alem(self) -> i64 {
        self.year as i64 + AMETE_ALEM_OFFSET
    }

    /// The year in the requested era.
    #[must_use]
    pub const fn year_in_era(self, era: Era) -> i64 {
        era.from_amete_mihret(self.year)
    }

    /// Whether the year is a leap year (Ṗagumēn has six days).
    #[must_use]
    pub const fn is_leap_year(self) -> bool {
        is_leap_year(self.year)
    }

    /// Days in this date's month.
    #[must_use]
    pub const fn days_in_month(self) -> u8 {
        days_in_month(self.year, self.month)
    }

    /// Days in this date's year: 365, or 366 in a leap year.
    #[must_use]
    pub const fn days_in_year(self) -> u16 {
        if is_leap_year(self.year) {
            366
        } else {
            365
        }
    }

    /// Day of the year, `1..=366`.
    #[must_use]
    pub const fn day_of_year(self) -> u16 {
        30 * (self.month as u16 - 1) + self.day as u16
    }

    /// Builds a date from a year and a day-of-year in `1..=366`.
    pub const fn from_year_day(year: i32, day_of_year: u16) -> Result<Self> {
        let max = if is_leap_year(year) { 366 } else { 365 };
        if day_of_year < 1 || day_of_year > max {
            return Err(Error::DayOutOfRange {
                day: day_of_year as i64,
                max: max as u8,
            });
        }
        let month = ((day_of_year - 1) / 30 + 1) as u8;
        let day = (day_of_year - 30 * (month as u16 - 1)) as u8;
        Ok(Self { year, month, day })
    }

    /// Converts to a fixed day number.
    ///
    /// `RD = epoch + 365·(y−1) + ⌊y/4⌋ + 30·(m−1) + d − 1`
    ///
    /// The `⌊y/4⌋` term — floor, not truncation — is what encodes the
    /// `y mod 4 == 3` leap phase *and* keeps it continuous through year 0.
    #[must_use]
    pub const fn to_rata_die(self) -> RataDie {
        let y = self.year as i64;
        RataDie::new(
            ETHIOPIC_EPOCH + 365 * (y - 1) + y.div_euclid(4) + 30 * (self.month as i64 - 1)
                + self.day as i64
                - 1,
        )
    }

    /// Converts from a fixed day number.
    pub fn from_rata_die(rd: RataDie) -> Result<Self> {
        let date = rd.check_range()?.get();
        let year = (4 * (date - ETHIOPIC_EPOCH) + 1463).div_euclid(1_461);
        if year < i64::from(i32::MIN) || year > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year });
        }
        let year32 = year as i32;
        let new_year = ETHIOPIC_EPOCH + 365 * (year - 1) + year.div_euclid(4);
        let month = ((date - new_year).div_euclid(30) + 1) as u8;
        let day = (date - (new_year + 30 * (i64::from(month) - 1)) + 1) as u8;
        Ok(Self {
            year: year32,
            month,
            day,
        })
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

    /// Converts to the proleptic Gregorian calendar.
    ///
    /// # Panics
    /// Never for dates constructible through the public API; the intermediate
    /// fixed day number is always in range.
    #[must_use]
    pub fn to_gregorian(self) -> GregorianDate {
        GregorianDate::from_rata_die(self.to_rata_die())
            .expect("Ethiopian dates always map into the Gregorian range")
    }

    /// Converts from the proleptic Gregorian calendar.
    pub fn from_gregorian(g: GregorianDate) -> Result<Self> {
        Self::from_rata_die(g.to_rata_die())
    }

    /// Converts to the proleptic Julian calendar.
    pub fn to_julian(self) -> Result<JulianDate> {
        JulianDate::from_rata_die(self.to_rata_die())
    }

    /// Converts from the proleptic Julian calendar.
    pub fn from_julian(j: JulianDate) -> Result<Self> {
        Self::from_rata_die(j.to_rata_die())
    }

    /// Days since the Unix epoch, for interop with `time_t`-style APIs.
    #[must_use]
    pub const fn to_unix_days(self) -> i64 {
        self.to_rata_die().to_unix_days()
    }

    /// Builds a date from a Unix timestamp in seconds, interpreted in UTC.
    ///
    /// For the local Ethiopian day (UTC+03:00, and optionally the dawn-based
    /// day boundary) use [`crate::clock::EthiopianDateTime`] instead.
    pub fn from_unix_timestamp(secs: i64) -> Result<Self> {
        Self::from_rata_die(RataDie::from_unix_timestamp(secs))
    }

    /// The day of the week.
    #[must_use]
    pub const fn weekday(self) -> Weekday {
        self.to_rata_die().weekday()
    }

    /// 1 Meskerem of this date's year (Enkutatash, the Ethiopian New Year).
    #[must_use]
    pub const fn new_year(self) -> Self {
        Self {
            year: self.year,
            month: 1,
            day: 1,
        }
    }

    /// Adds a signed number of days.
    pub fn add_days(self, days: i64) -> Result<Self> {
        let rd = self
            .to_rata_die()
            .checked_add_days(days)
            .ok_or(Error::Overflow)?;
        Self::from_rata_die(rd)
    }

    /// Subtracts a signed number of days.
    pub fn sub_days(self, days: i64) -> Result<Self> {
        self.add_days(-days)
    }

    /// The next day.
    pub fn succ(self) -> Result<Self> {
        self.add_days(1)
    }

    /// The previous day.
    pub fn pred(self) -> Result<Self> {
        self.add_days(-1)
    }

    /// Adds months, clamping the day to the length of the target month.
    ///
    /// Clamping matters here because Ṗagumēn is five or six days long: adding
    /// one month to 30 Nehase lands on Ṗagumēn 5 (or 6), not on a nonexistent
    /// "Ṗagumēn 30".
    pub fn add_months(self, months: i64) -> Result<Self> {
        let total = i64::from(self.year) * 13 + i64::from(self.month - 1) + months;
        let year = total.div_euclid(13);
        let month = (total.rem_euclid(13) + 1) as u8;
        if year < i64::from(i32::MIN) || year > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year });
        }
        let year = year as i32;
        let max = days_in_month(year, month);
        let day = if self.day > max { max } else { self.day };
        Self::from_ymd(year, month, day)
    }

    /// Adds years, clamping 6 Ṗagumēn in a leap year down to 5 Ṗagumēn.
    pub fn add_years(self, years: i64) -> Result<Self> {
        let year = i64::from(self.year) + years;
        if year < i64::from(i32::MIN) || year > i64::from(i32::MAX) {
            return Err(Error::YearOutOfRange { year });
        }
        let year = year as i32;
        let max = days_in_month(year, self.month);
        let day = if self.day > max { max } else { self.day };
        Self::from_ymd(year, self.month, day)
    }

    /// Number of days from `other` to `self`.
    #[must_use]
    pub const fn days_since(self, other: Self) -> i64 {
        self.to_rata_die().get() - other.to_rata_die().get()
    }
}

/// Whether an *Amete Mihret* year is an Ethiopian leap year.
///
/// The rule is `y mod 4 == 3`, using Euclidean remainder so that it stays
/// correct for `y <= 0`.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year as i64).rem_euclid(4) == 3
}

/// Number of days in an Ethiopian month: always 30, except Ṗagumēn.
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1..=12 => 30,
        13 => {
            if is_leap_year(year) {
                6
            } else {
                5
            }
        }
        _ => 0,
    }
}

/// Number of days in an Ethiopian year.
#[must_use]
pub const fn days_in_year(year: i32) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

impl fmt::Display for EthiopianDate {
    /// `2017-01-01`, zero padded, sign for negative years — the same shape as
    /// ISO 8601 so that the CLI can round-trip through shell pipelines.
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

impl FromStr for EthiopianDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (y, m, d) = crate::parse::parse_ymd(s)?;
        if !(1..=13).contains(&m) {
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

impl TryFrom<RataDie> for EthiopianDate {
    type Error = Error;
    fn try_from(rd: RataDie) -> Result<Self> {
        Self::from_rata_die(rd)
    }
}

impl From<EthiopianDate> for RataDie {
    fn from(d: EthiopianDate) -> Self {
        d.to_rata_die()
    }
}

impl TryFrom<GregorianDate> for EthiopianDate {
    type Error = Error;
    fn try_from(g: GregorianDate) -> Result<Self> {
        Self::from_gregorian(g)
    }
}

impl From<EthiopianDate> for GregorianDate {
    fn from(e: EthiopianDate) -> Self {
        e.to_gregorian()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_29_august_8_ce_julian() {
        let e = EthiopianDate::from_ymd(1, 1, 1).unwrap();
        assert_eq!(e.to_rata_die().get(), ETHIOPIC_EPOCH);
        assert_eq!(e.to_jdn(), 1_724_221);
        assert_eq!(e.to_julian().unwrap().as_tuple(), (8, 8, 29));
        assert_eq!(e.to_gregorian().as_tuple(), (8, 8, 27));
    }

    #[test]
    fn enkutatash_anchors() {
        // Published Ethiopian New Year dates.
        let cases = [
            (2000, (2007, 9, 12)), // the Ethiopian Millennium
            (2013, (2020, 9, 11)),
            (2014, (2021, 9, 11)),
            (2015, (2022, 9, 11)),
            (2016, (2023, 9, 12)), // follows the leap year 2015 E.C.
            (2017, (2024, 9, 11)),
            (2018, (2025, 9, 11)),
            (2019, (2026, 9, 11)),
            (2020, (2027, 9, 12)), // follows the leap year 2019 E.C.
        ];
        for (ec, greg) in cases {
            let e = EthiopianDate::from_ymd(ec, 1, 1).unwrap();
            assert_eq!(e.to_gregorian().as_tuple(), greg, "new year of {ec} E.C.");
        }
    }

    #[test]
    fn leap_phase_is_three_mod_four() {
        assert!(is_leap_year(2015));
        assert!(is_leap_year(2019));
        assert!(!is_leap_year(2016));
        assert!(!is_leap_year(2000));
        // Continuous across year 0.
        assert!(is_leap_year(-1));
        assert!(is_leap_year(3));
        assert!(!is_leap_year(0));
        assert!(!is_leap_year(-2));
        // The leap day precedes the Gregorian leap year by ~5 months.
        let p = EthiopianDate::from_ymd(2015, 13, 6).unwrap();
        assert_eq!(p.to_gregorian().as_tuple(), (2023, 9, 11));
        assert!(crate::gregorian::is_leap_year(2024));
    }

    #[test]
    fn pagume_length() {
        assert_eq!(days_in_month(2015, 13), 6);
        assert_eq!(days_in_month(2016, 13), 5);
        assert!(EthiopianDate::from_ymd(2016, 13, 6).is_err());
        assert!(EthiopianDate::from_ymd(2015, 13, 6).is_ok());
        assert!(EthiopianDate::from_ymd(2015, 13, 7).is_err());
        assert!(EthiopianDate::from_ymd(2015, 1, 31).is_err());
    }

    #[test]
    fn roundtrip_dense_over_two_millennia() {
        let start = EthiopianDate::from_ymd(1, 1, 1).unwrap().to_rata_die().get();
        let end = EthiopianDate::from_ymd(2200, 1, 1).unwrap().to_rata_die().get();
        for rd in start..end {
            let d = EthiopianDate::from_rata_die(RataDie::new(rd)).unwrap();
            assert_eq!(d.to_rata_die().get(), rd, "roundtrip failed at rd={rd}");
            assert!(d.month >= 1 && d.month <= 13);
            assert!(d.day >= 1 && d.day <= d.days_in_month());
        }
    }

    #[test]
    fn roundtrip_before_the_epoch() {
        let start = EthiopianDate::from_ymd(-500, 1, 1).unwrap().to_rata_die().get();
        let end = EthiopianDate::from_ymd(30, 1, 1).unwrap().to_rata_die().get();
        for rd in start..end {
            let d = EthiopianDate::from_rata_die(RataDie::new(rd)).unwrap();
            assert_eq!(d.to_rata_die().get(), rd);
            assert!(d.day >= 1 && d.day <= d.days_in_month());
        }
    }

    #[test]
    fn day_of_year_roundtrip() {
        for year in [2014, 2015, 2016, 2017, -1, 0, 1] {
            let n = days_in_year(year);
            for doy in 1..=n {
                let d = EthiopianDate::from_year_day(year, doy).unwrap();
                assert_eq!(d.day_of_year(), doy);
                assert_eq!(d.year(), year);
            }
            assert!(EthiopianDate::from_year_day(year, n + 1).is_err());
            assert!(EthiopianDate::from_year_day(year, 0).is_err());
        }
    }

    #[test]
    fn month_arithmetic_clamps_into_pagume() {
        let d = EthiopianDate::from_ymd(2016, 12, 30).unwrap();
        assert_eq!(d.add_months(1).unwrap().as_tuple(), (2016, 13, 5));
        let d = EthiopianDate::from_ymd(2015, 12, 30).unwrap();
        assert_eq!(d.add_months(1).unwrap().as_tuple(), (2015, 13, 6));
        // Rolling over the year boundary.
        assert_eq!(d.add_months(2).unwrap().as_tuple(), (2016, 1, 30));
        assert_eq!(d.add_months(-12).unwrap().as_tuple(), (2014, 12, 30));
        // Leap day + 1 year clamps.
        let p = EthiopianDate::from_ymd(2015, 13, 6).unwrap();
        assert_eq!(p.add_years(1).unwrap().as_tuple(), (2016, 13, 5));
        assert_eq!(p.add_years(4).unwrap().as_tuple(), (2019, 13, 6));
    }

    #[test]
    fn ordering_is_chronological() {
        let a = EthiopianDate::from_ymd(2016, 13, 5).unwrap();
        let b = EthiopianDate::from_ymd(2017, 1, 1).unwrap();
        assert!(a < b);
        assert_eq!(b.days_since(a), 1);
        assert_eq!(a.succ().unwrap(), b);
        assert_eq!(b.pred().unwrap(), a);
    }

    #[test]
    fn parsing_and_display() {
        assert_eq!(
            "2017-13-05".parse::<EthiopianDate>().unwrap().as_tuple(),
            (2017, 13, 5)
        );
        assert!("2016-13-06".parse::<EthiopianDate>().is_err());
        assert!("2017-14-01".parse::<EthiopianDate>().is_err());
    }

    #[test]
    fn amete_alem_era() {
        let d = EthiopianDate::from_ymd(2017, 1, 1).unwrap();
        assert_eq!(d.amete_alem(), 7517);
        assert_eq!(
            EthiopianDate::from_ymd_era(7517, 1, 1, Era::AmeteAlem).unwrap(),
            d
        );
    }
}
