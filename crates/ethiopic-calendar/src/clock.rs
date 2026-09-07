//! **Ethiopian clock time** — a deliberately separate, opt-in layer.
//!
//! Ethiopian time keeping counts hours from *dawn*, not from midnight. Two
//! twelve-hour cycles run 06:00→18:00 and 18:00→06:00, so the Ethiopian hour is
//! the Western hour shifted by six:
//!
//! ```text
//! 06:00 Western  =  12:00 Ethiopian (ጠዋት)   ← the day begins
//! 07:00          =   1:00 (ጠዋት)
//! 12:00          =   6:00 (ከሰዓት)
//! 18:00          =  12:00 (ማታ)
//! 00:00          =   6:00 (ለሊት)
//! ```
//!
//! # Why this is a separate layer
//!
//! Date arithmetic must not silently depend on a clock, a time zone, or a
//! sunrise model. [`crate::EthiopianDate`] is a pure day count; everything in
//! this module is opt-in on top of it. In particular:
//!
//! * **Time zone.** Ethiopia uses East Africa Time, UTC+03:00, and has never
//!   observed daylight saving. Before 1936 Addis Ababa kept LMT (UTC+02:34:48);
//!   this crate models the modern fixed offset only, and says so rather than
//!   pretending to a precision it does not have.
//! * **Day boundary.** Traditionally the day turns at dawn, so 02:00 on Tuesday
//!   Western is still *Monday night* (ለሊት) in Ethiopian reckoning. Civil and
//!   administrative use has adopted the midnight boundary. Neither is "the"
//!   answer, so [`DayStart`] makes the caller choose, and the default matches
//!   the traditional reckoning that the hour names imply.
//! * **Sunrise.** The six-hour offset is a *convention* anchored on nominal
//!   6 a.m. dawn, which is a good approximation near the equator (Addis Ababa's
//!   sunrise varies only between about 06:00 and 06:40 across the year). This
//!   module does not compute true solar sunrise; if you need that, feed an
//!   astronomical library's result into [`EthiopianTime::from_wall_clock`]
//!   yourself.

use core::fmt;

use crate::error::{Error, Result};
use crate::ethiopic::EthiopianDate;
use crate::rata_die::RataDie;

/// East Africa Time offset from UTC, in seconds (+03:00, no DST).
pub const EAT_UTC_OFFSET_SECONDS: i32 = 3 * 3600;

/// Hours the Ethiopian clock is shifted from the Western one.
pub const ETHIOPIAN_HOUR_OFFSET: i64 = 6;

/// Which instant starts a new Ethiopian calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum DayStart {
    /// Traditional reckoning: the day turns at 06:00 local time (dawn).
    #[default]
    Dawn,
    /// Civil reckoning: the day turns at 00:00 local time.
    Midnight,
}

/// The four traditional divisions of the Ethiopian day, each six hours long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// ጠዋት — morning, 06:00–11:59 Western.
    Tewat,
    /// ከሰዓት — afternoon, 12:00–17:59 Western.
    Keseat,
    /// ማታ — evening, 18:00–23:59 Western.
    Mata,
    /// ለሊት — night, 00:00–05:59 Western.
    Lelit,
}

impl Period {
    /// The period containing a Western hour `0..=23`.
    #[must_use]
    pub const fn from_wall_hour(hour: u8) -> Self {
        match hour {
            6..=11 => Self::Tewat,
            12..=17 => Self::Keseat,
            18..=23 => Self::Mata,
            _ => Self::Lelit,
        }
    }

    /// Amharic name.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::Tewat => "ጠዋት",
            Self::Keseat => "ከሰዓት",
            Self::Mata => "ማታ",
            Self::Lelit => "ለሊት",
        }
    }

    /// Romanised name.
    #[must_use]
    pub const fn transliterated(self) -> &'static str {
        match self {
            Self::Tewat => "tewat",
            Self::Keseat => "keseat",
            Self::Mata => "mata",
            Self::Lelit => "lelit",
        }
    }

    /// English gloss.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::Tewat => "morning",
            Self::Keseat => "afternoon",
            Self::Mata => "evening",
            Self::Lelit => "night",
        }
    }

    /// Whether this period belongs to the daylight half of the day.
    #[must_use]
    pub const fn is_daytime(self) -> bool {
        matches!(self, Self::Tewat | Self::Keseat)
    }
}

/// A time of day, stored as Western wall-clock time and read out either way.
///
/// ```
/// use ethiopic_calendar::clock::{EthiopianTime, Period};
///
/// let t = EthiopianTime::from_wall_clock(9, 30, 0).unwrap();
/// assert_eq!(t.ethiopian_hour(), 3);          // 9 a.m. is "3 o'clock"
/// assert_eq!(t.period(), Period::Tewat);      // in the morning
/// assert_eq!(t.wall_hour(), 9);
///
/// // Dawn is twelve o'clock, not zero: Ethiopian hours are 1..=12.
/// assert_eq!(EthiopianTime::from_wall_clock(6, 0, 0).unwrap().ethiopian_hour(), 12);
/// assert_eq!(EthiopianTime::from_wall_clock(18, 0, 0).unwrap().ethiopian_hour(), 12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EthiopianTime {
    hour: u8,
    minute: u8,
    second: u8,
}

impl EthiopianTime {
    /// Builds a time from Western 24-hour wall-clock components.
    pub const fn from_wall_clock(hour: u8, minute: u8, second: u8) -> Result<Self> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(Error::TimeOutOfRange);
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }

    /// Builds a time from an Ethiopian hour `1..=12` plus the period of day.
    ///
    /// Not every (hour, period) pair exists: the hours 12, 1, 2, 3, 4, 5 occur
    /// only in ጠዋት and ማታ, and 6, 7, 8, 9, 10, 11 only in ከሰዓት and ለሊት. A
    /// combination such as "3 o'clock ከሰዓት" is not a time of day, and is
    /// rejected rather than silently coerced.
    ///
    /// ```
    /// use ethiopic_calendar::clock::{EthiopianTime, Period};
    /// // 3 o'clock in the morning = 09:00
    /// assert_eq!(EthiopianTime::from_ethiopian(3, 30, 0, Period::Tewat).unwrap().wall_hour(), 9);
    /// // 8 o'clock in the afternoon = 14:00
    /// assert_eq!(EthiopianTime::from_ethiopian(8, 0, 0, Period::Keseat).unwrap().wall_hour(), 14);
    /// // 6 o'clock at night = midnight
    /// assert_eq!(EthiopianTime::from_ethiopian(6, 0, 0, Period::Lelit).unwrap().wall_hour(), 0);
    /// // ...but "3 o'clock in the afternoon" does not exist.
    /// assert!(EthiopianTime::from_ethiopian(3, 0, 0, Period::Keseat).is_err());
    /// ```
    pub const fn from_ethiopian(hour: u8, minute: u8, second: u8, period: Period) -> Result<Self> {
        if hour < 1 || hour > 12 || minute > 59 || second > 59 {
            return Err(Error::TimeOutOfRange);
        }
        let base: u8 = match period {
            Period::Lelit => 0,
            Period::Tewat => 6,
            Period::Keseat => 12,
            Period::Mata => 18,
        };
        // The Western hour is congruent to (ethiopian + 6) mod 12; exactly one
        // representative of that residue class falls inside the six-hour block
        // of a period — or none, for an impossible combination.
        let mut wall = (hour as u16 + ETHIOPIAN_HOUR_OFFSET as u16) % 12;
        while wall < base as u16 {
            wall += 12;
        }
        if wall > base as u16 + 5 {
            return Err(Error::TimeOutOfRange);
        }
        Self::from_wall_clock(wall as u8, minute, second)
    }

    /// Builds a time from seconds since local midnight.
    pub const fn from_seconds_since_midnight(secs: u32) -> Result<Self> {
        if secs >= 86_400 {
            return Err(Error::TimeOutOfRange);
        }
        Self::from_wall_clock(
            (secs / 3600) as u8,
            ((secs / 60) % 60) as u8,
            (secs % 60) as u8,
        )
    }

    /// Western hour, `0..=23`.
    #[must_use]
    pub const fn wall_hour(self) -> u8 {
        self.hour
    }

    /// Minute, `0..=59`.
    #[must_use]
    pub const fn minute(self) -> u8 {
        self.minute
    }

    /// Second, `0..=59`.
    #[must_use]
    pub const fn second(self) -> u8 {
        self.second
    }

    /// Seconds since local midnight.
    #[must_use]
    pub const fn seconds_since_midnight(self) -> u32 {
        self.hour as u32 * 3600 + self.minute as u32 * 60 + self.second as u32
    }

    /// The Ethiopian hour, `1..=12`.
    ///
    /// `(wall + 6) mod 12`, with 0 rendered as 12 — the hour after 11 is 12,
    /// not 0, and the hour at dawn is 12, not 0.
    #[must_use]
    pub const fn ethiopian_hour(self) -> u8 {
        let h = (self.hour as i64 + ETHIOPIAN_HOUR_OFFSET).rem_euclid(12);
        if h == 0 {
            12
        } else {
            h as u8
        }
    }

    /// The period of the day.
    #[must_use]
    pub const fn period(self) -> Period {
        Period::from_wall_hour(self.hour)
    }

    /// Whether this time is before the traditional dawn day boundary, i.e.
    /// whether it still belongs to the previous Ethiopian day.
    #[must_use]
    pub const fn is_before_dawn(self) -> bool {
        (self.hour as i64) < ETHIOPIAN_HOUR_OFFSET
    }
}

impl fmt::Display for EthiopianTime {
    /// Ethiopian reading, e.g. `3:30:00 ጠዋት`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{:02}:{:02} {}",
            self.ethiopian_hour(),
            self.minute,
            self.second,
            self.period().amharic()
        )
    }
}

/// A date plus a time of day, in a fixed-offset zone, with an explicit choice
/// of day boundary.
///
/// ```
/// use ethiopic_calendar::clock::{DayStart, EthiopianDateTime, EAT_UTC_OFFSET_SECONDS};
///
/// // 2025-09-11T03:30:00Z = 06:30 in Addis Ababa = Meskerem 1, 2018, 12:30 ጠዋት
/// let dt = EthiopianDateTime::from_unix_timestamp(
///     1_757_561_400, EAT_UTC_OFFSET_SECONDS, DayStart::Dawn).unwrap();
/// assert_eq!(dt.date().as_tuple(), (2018, 1, 1));
/// assert_eq!(dt.time().ethiopian_hour(), 12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthiopianDateTime {
    date: EthiopianDate,
    time: EthiopianTime,
    utc_offset_seconds: i32,
    day_start: DayStart,
}

impl EthiopianDateTime {
    /// Combines a date and time explicitly.
    #[must_use]
    pub const fn new(
        date: EthiopianDate,
        time: EthiopianTime,
        utc_offset_seconds: i32,
        day_start: DayStart,
    ) -> Self {
        Self {
            date,
            time,
            utc_offset_seconds,
            day_start,
        }
    }

    /// Converts a Unix timestamp into local Ethiopian date and time.
    ///
    /// With [`DayStart::Dawn`], instants between local midnight and 06:00 are
    /// reported as belonging to the *previous* Ethiopian day, matching the
    /// traditional reckoning that the hour names presuppose.
    pub fn from_unix_timestamp(
        secs: i64,
        utc_offset_seconds: i32,
        day_start: DayStart,
    ) -> Result<Self> {
        let local = secs
            .checked_add(i64::from(utc_offset_seconds))
            .ok_or(Error::Overflow)?;
        let day = local.div_euclid(86_400);
        let sod = local.rem_euclid(86_400) as u32;
        let time = EthiopianTime::from_seconds_since_midnight(sod)?;
        let mut rd = RataDie::from_unix_days(day);
        if day_start == DayStart::Dawn && time.is_before_dawn() {
            rd -= 1;
        }
        Ok(Self {
            date: EthiopianDate::from_rata_die(rd)?,
            time,
            utc_offset_seconds,
            day_start,
        })
    }

    /// The Ethiopian date, already adjusted for the chosen day boundary.
    #[must_use]
    pub const fn date(&self) -> EthiopianDate {
        self.date
    }

    /// The time of day.
    #[must_use]
    pub const fn time(&self) -> EthiopianTime {
        self.time
    }

    /// The UTC offset in seconds used for the conversion.
    #[must_use]
    pub const fn utc_offset_seconds(&self) -> i32 {
        self.utc_offset_seconds
    }

    /// Which day boundary was applied.
    #[must_use]
    pub const fn day_start(&self) -> DayStart {
        self.day_start
    }

    /// The same instant expressed as a Unix timestamp.
    #[must_use]
    pub fn to_unix_timestamp(&self) -> i64 {
        let mut rd = self.date.to_rata_die();
        if self.day_start == DayStart::Dawn && self.time.is_before_dawn() {
            rd += 1;
        }
        rd.to_unix_timestamp() + i64::from(self.time.seconds_since_midnight())
            - i64::from(self.utc_offset_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hour_conversion_table() {
        // The canonical table every Ethiopian schoolchild knows.
        let cases = [
            (0u8, 6u8, Period::Lelit),
            (1, 7, Period::Lelit),
            (5, 11, Period::Lelit),
            (6, 12, Period::Tewat),
            (7, 1, Period::Tewat),
            (9, 3, Period::Tewat),
            (11, 5, Period::Tewat),
            (12, 6, Period::Keseat),
            (13, 7, Period::Keseat),
            (17, 11, Period::Keseat),
            (18, 12, Period::Mata),
            (19, 1, Period::Mata),
            (23, 5, Period::Mata),
        ];
        for (wall, eth, period) in cases {
            let t = EthiopianTime::from_wall_clock(wall, 0, 0).unwrap();
            assert_eq!(t.ethiopian_hour(), eth, "wall {wall}");
            assert_eq!(t.period(), period, "wall {wall}");
        }
    }

    #[test]
    fn ethiopian_to_wall_roundtrip() {
        for wall in 0..24u8 {
            let t = EthiopianTime::from_wall_clock(wall, 17, 42).unwrap();
            let back =
                EthiopianTime::from_ethiopian(t.ethiopian_hour(), 17, 42, t.period()).unwrap();
            assert_eq!(back, t, "wall hour {wall}");
        }
    }

    #[test]
    fn rejects_impossible_times() {
        assert_eq!(
            EthiopianTime::from_wall_clock(24, 0, 0),
            Err(Error::TimeOutOfRange)
        );
        assert!(EthiopianTime::from_wall_clock(23, 60, 0).is_err());
        assert!(EthiopianTime::from_ethiopian(0, 0, 0, Period::Tewat).is_err());
        assert!(EthiopianTime::from_ethiopian(13, 0, 0, Period::Tewat).is_err());
        // Impossible hour/period combinations.
        assert!(EthiopianTime::from_ethiopian(3, 0, 0, Period::Keseat).is_err());
        assert!(EthiopianTime::from_ethiopian(3, 0, 0, Period::Lelit).is_err());
        assert!(EthiopianTime::from_ethiopian(9, 0, 0, Period::Tewat).is_err());
        assert!(EthiopianTime::from_ethiopian(9, 0, 0, Period::Mata).is_err());
        assert!(EthiopianTime::from_seconds_since_midnight(86_400).is_err());
    }

    #[test]
    fn dawn_boundary_moves_the_date_back() {
        // 2025-09-11T00:30:00+03:00 — after midnight, before dawn.
        let midnightish = EthiopianDate::from_ymd(2018, 1, 1)
            .unwrap()
            .to_rata_die()
            .to_unix_timestamp()
            + 30 * 60
            - i64::from(EAT_UTC_OFFSET_SECONDS);
        let dawn = EthiopianDateTime::from_unix_timestamp(
            midnightish,
            EAT_UTC_OFFSET_SECONDS,
            DayStart::Dawn,
        )
        .unwrap();
        let civil = EthiopianDateTime::from_unix_timestamp(
            midnightish,
            EAT_UTC_OFFSET_SECONDS,
            DayStart::Midnight,
        )
        .unwrap();
        assert_eq!(civil.date().as_tuple(), (2018, 1, 1));
        assert_eq!(dawn.date().as_tuple(), (2017, 13, 5));
        assert_eq!(dawn.time().ethiopian_hour(), 6);
        assert_eq!(dawn.time().period(), Period::Lelit);
    }

    #[test]
    fn timestamp_roundtrip_every_ten_minutes_for_a_week() {
        let base = 1_757_500_000i64;
        for k in 0..1008 {
            let ts = base + k * 600;
            for day_start in [DayStart::Dawn, DayStart::Midnight] {
                let dt =
                    EthiopianDateTime::from_unix_timestamp(ts, EAT_UTC_OFFSET_SECONDS, day_start)
                        .unwrap();
                assert_eq!(dt.to_unix_timestamp(), ts, "ts={ts} {day_start:?}");
            }
        }
    }

    #[test]
    fn periods_are_six_hours_each() {
        let mut counts = [0u8; 4];
        for h in 0..24u8 {
            match Period::from_wall_hour(h) {
                Period::Tewat => counts[0] += 1,
                Period::Keseat => counts[1] += 1,
                Period::Mata => counts[2] += 1,
                Period::Lelit => counts[3] += 1,
            }
        }
        assert_eq!(counts, [6, 6, 6, 6]);
    }
}
