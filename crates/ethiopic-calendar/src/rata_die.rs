//! [`RataDie`] — the crate's pivot representation: a *fixed day number*.
//!
//! Every calendar in this crate converts to and from R.D. and never directly to
//! another calendar. That single-pivot design is what keeps the conversion
//! matrix consistent: with `n` calendars you write `2n` functions instead of
//! `n(n-1)` pairwise ones, and there is exactly one place where an off-by-one
//! can hide.
//!
//! R.D. 1 is 1 January 1 CE in the **proleptic Gregorian** calendar
//! (Reingold & Dershowitz, *Calendrical Calculations*, §1.2).

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::error::{Error, Result};
use crate::weekday::Weekday;

/// Number of days between the R.D. epoch and the Julian Day Number epoch.
///
/// `JDN = RD + 1_721_425`, where JDN is the integer day number that *starts at
/// noon UT* (Meeus, *Astronomical Algorithms*, §7). The half-day offset is why
/// the constant is `1_721_425` and not `1_721_424`; getting this wrong is the
/// single most common bug in calendar libraries, and it shows up as an
/// intermittent one-day error depending on the time of day.
pub const JDN_OFFSET: i64 = 1_721_425;

/// R.D. of 1 January 1970 (the Unix epoch).
pub const UNIX_EPOCH_RD: i64 = 719_163;

/// Offset between R.D. and the Modified Julian Date: `MJD = RD - 678_576`.
pub const MJD_OFFSET: i64 = 678_576;

/// A fixed day number ("Rata Die"), counting days elapsed since
/// 31 December 1 BCE (proleptic Gregorian), i.e. R.D. 1 = 1 January 1 CE.
///
/// ```
/// use ethiopic_calendar::{GregorianDate, RataDie};
///
/// let rd = GregorianDate::from_ymd(1, 1, 1).unwrap().to_rata_die();
/// assert_eq!(rd.get(), 1);
/// assert_eq!(rd.to_jdn(), 1_721_426);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct RataDie(i64);

impl RataDie {
    /// Constructs a fixed day number from a raw count.
    #[must_use]
    pub const fn new(days: i64) -> Self {
        Self(days)
    }

    /// The raw day count.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Converts to a Julian Day Number (integer, noon-based).
    #[must_use]
    pub const fn to_jdn(self) -> i64 {
        self.0 + JDN_OFFSET
    }

    /// Builds a fixed day number from an integer Julian Day Number.
    #[must_use]
    pub const fn from_jdn(jdn: i64) -> Self {
        Self(jdn - JDN_OFFSET)
    }

    /// Converts to a Julian Date at midnight, i.e. `JDN - 0.5`.
    ///
    /// Provided because astronomical code usually wants the fractional form and
    /// naive `as f64` casts of a JDN are a classic source of half-day errors.
    ///
    /// Beyond about ±2^53 days the `f64` result is no longer exact; that is
    /// nine quadrillion years and far outside the calendrical range, but it is
    /// the reason this returns a float only on request rather than being the
    /// primary representation.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_julian_date_midnight(self) -> f64 {
        self.to_jdn() as f64 - 0.5
    }

    /// Converts to a Modified Julian Date (integer day).
    #[must_use]
    pub const fn to_mjd(self) -> i64 {
        self.0 - MJD_OFFSET
    }

    /// Builds a fixed day number from a Modified Julian Date.
    #[must_use]
    pub const fn from_mjd(mjd: i64) -> Self {
        Self(mjd + MJD_OFFSET)
    }

    /// Days since the Unix epoch (1970-01-01), negative before it.
    #[must_use]
    pub const fn to_unix_days(self) -> i64 {
        self.0 - UNIX_EPOCH_RD
    }

    /// Builds a fixed day number from days since the Unix epoch.
    #[must_use]
    pub const fn from_unix_days(days: i64) -> Self {
        Self(days + UNIX_EPOCH_RD)
    }

    /// Builds a fixed day number from a Unix timestamp in seconds, interpreted
    /// in UTC. Uses flooring division so pre-1970 timestamps are correct.
    #[must_use]
    pub const fn from_unix_timestamp(secs: i64) -> Self {
        Self(secs.div_euclid(86_400) + UNIX_EPOCH_RD)
    }

    /// Seconds from the Unix epoch to midnight UTC of this day.
    #[must_use]
    pub const fn to_unix_timestamp(self) -> i64 {
        self.to_unix_days() * 86_400
    }

    /// Day of the week. R.D. 1 (1 January 1 CE, proleptic Gregorian) was a Monday.
    #[must_use]
    pub const fn weekday(self) -> Weekday {
        Weekday::from_days_since_sunday(self.0)
    }

    /// The `k`-day (weekday) on or before this date.
    #[must_use]
    pub const fn kday_on_or_before(self, k: Weekday) -> Self {
        Self(self.0 - (self.0 - k as i64).rem_euclid(7))
    }

    /// The `k`-day strictly before this date.
    #[must_use]
    pub const fn kday_before(self, k: Weekday) -> Self {
        Self(self.0 - 1).kday_on_or_before(k)
    }

    /// The `k`-day on or after this date.
    #[must_use]
    pub const fn kday_on_or_after(self, k: Weekday) -> Self {
        Self(self.0 + 6).kday_on_or_before(k)
    }

    /// The `k`-day strictly after this date.
    #[must_use]
    pub const fn kday_after(self, k: Weekday) -> Self {
        Self(self.0 + 7).kday_on_or_before(k)
    }

    /// Adds a number of days, returning `None` on `i64` overflow.
    #[must_use]
    pub const fn checked_add_days(self, days: i64) -> Option<Self> {
        match self.0.checked_add(days) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Days from `other` to `self` (`self - other`).
    #[must_use]
    pub const fn days_since(self, other: Self) -> i64 {
        self.0 - other.0
    }

    /// Fails if the fixed day number is outside the range this crate supports
    /// for calendar conversion (roughly ±5.8 million years, the point at which
    /// an [`i32`] year would overflow).
    pub const fn check_range(self) -> Result<Self> {
        if self.0 < MIN_RATA_DIE || self.0 > MAX_RATA_DIE {
            Err(Error::RataDieOutOfRange { rata_die: self.0 })
        } else {
            Ok(self)
        }
    }
}

/// Coarse guard rail: the largest magnitude fixed day number this crate will
/// even attempt to convert, `(2^31 - 1) * 366`.
///
/// It is deliberately *wider* than the exact `i32`-year bound; the precise
/// check lives in each calendar's `from_rata_die`, which computes the year in
/// [`i64`] and rejects anything that does not fit an [`i32`]. Having both means
/// the arithmetic below can never come near `i64` overflow (the widest
/// intermediate is `4 * rd`, about 3.1e12, against an `i64` ceiling of 9.2e18).
pub const MAX_RATA_DIE: i64 = 2_147_483_647 * 366;

/// Mirror of [`MAX_RATA_DIE`] on the negative side.
pub const MIN_RATA_DIE: i64 = -MAX_RATA_DIE;

impl fmt::Display for RataDie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<i64> for RataDie {
    type Output = Self;
    fn add(self, rhs: i64) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<i64> for RataDie {
    fn add_assign(&mut self, rhs: i64) {
        self.0 += rhs;
    }
}

impl Sub<i64> for RataDie {
    type Output = Self;
    fn sub(self, rhs: i64) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<i64> for RataDie {
    fn sub_assign(&mut self, rhs: i64) {
        self.0 -= rhs;
    }
}

impl Sub<RataDie> for RataDie {
    type Output = i64;
    fn sub(self, rhs: Self) -> i64 {
        self.0 - rhs.0
    }
}

impl From<RataDie> for i64 {
    fn from(rd: RataDie) -> Self {
        rd.0
    }
}

impl From<i64> for RataDie {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jdn_offset_is_noon_based() {
        // 1 January 2000 (Gregorian) = JDN 2451545 (the J2000.0 epoch, at noon).
        let rd = RataDie::new(730_120);
        assert_eq!(rd.to_jdn(), 2_451_545);
        assert_eq!(RataDie::from_jdn(2_451_545), rd);
        assert!((rd.to_julian_date_midnight() - 2_451_544.5).abs() < 1e-9);
    }

    #[test]
    fn unix_and_mjd_anchors() {
        let rd = RataDie::new(UNIX_EPOCH_RD);
        assert_eq!(rd.to_unix_days(), 0);
        assert_eq!(rd.to_mjd(), 40_587);
        assert_eq!(RataDie::from_mjd(40_587), rd);
        assert_eq!(RataDie::from_unix_timestamp(0), rd);
        // Flooring, not truncating, division for pre-epoch timestamps.
        assert_eq!(RataDie::from_unix_timestamp(-1).to_unix_days(), -1);
        assert_eq!(RataDie::from_unix_timestamp(-86_400).to_unix_days(), -1);
        assert_eq!(RataDie::from_unix_timestamp(-86_401).to_unix_days(), -2);
    }

    #[test]
    fn rd_one_is_monday() {
        assert_eq!(RataDie::new(1).weekday(), Weekday::Monday);
        assert_eq!(RataDie::new(0).weekday(), Weekday::Sunday);
        assert_eq!(RataDie::new(-1).weekday(), Weekday::Saturday);
        assert_eq!(RataDie::new(-7).weekday(), Weekday::Sunday);
        assert_eq!(RataDie::new(-8).weekday(), Weekday::Saturday);
    }

    #[test]
    fn kday_family() {
        // R.D. 730120 = 2000-01-01, a Saturday.
        let rd = RataDie::new(730_120);
        assert_eq!(rd.weekday(), Weekday::Saturday);
        assert_eq!(rd.kday_on_or_before(Weekday::Saturday), rd);
        assert_eq!(rd.kday_before(Weekday::Saturday), rd - 7);
        assert_eq!(rd.kday_on_or_after(Weekday::Saturday), rd);
        assert_eq!(rd.kday_after(Weekday::Saturday), rd + 7);
        assert_eq!(rd.kday_on_or_before(Weekday::Sunday), rd - 6);
        assert_eq!(rd.kday_after(Weekday::Sunday), rd + 1);
    }
}
