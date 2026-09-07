//! Error type shared by every conversion and parsing entry point.

use core::fmt;

/// Convenience alias used throughout the crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Everything that can go wrong when constructing or converting a date.
///
/// The variants deliberately carry the offending value so that callers (and the
/// `ethdate` CLI) can produce actionable messages without re-parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Error {
    /// Month index outside the calendar's valid range.
    ///
    /// Ethiopic: `1..=13`. Gregorian / Julian: `1..=12`.
    MonthOutOfRange {
        /// The month that was supplied.
        month: i64,
    },
    /// Day-of-month outside the valid range *for that particular month and year*.
    ///
    /// This is what catches `Ṗagumēn 6` in a common year and `February 29` in a
    /// non-leap Gregorian year.
    DayOutOfRange {
        /// The day that was supplied.
        day: i64,
        /// The number of days the month actually has.
        max: u8,
    },
    /// Year outside the range representable by [`i32`] after conversion.
    YearOutOfRange {
        /// The year that was supplied (as computed in `i64`).
        year: i64,
    },
    /// The Julian calendar has no year zero: 1 BCE is written `-1`.
    JulianYearZero,
    /// A [`crate::RataDie`] that cannot be represented in the target calendar
    /// without overflowing the [`i32`] year.
    RataDieOutOfRange {
        /// The offending fixed-day number.
        rata_die: i64,
    },
    /// Arithmetic would overflow the supported range.
    Overflow,
    /// A string could not be parsed as a date.
    Parse(ParseError),
    /// Clock time outside `00:00:00 ..= 23:59:59`.
    TimeOutOfRange,
}

/// Detail for [`Error::Parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ParseError {
    /// The input did not have the shape `YYYY-MM-DD`.
    Shape,
    /// A field was not a valid integer.
    NotANumber,
    /// The input was empty.
    Empty,
    /// Trailing characters after a complete date.
    Trailing,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MonthOutOfRange { month } => write!(f, "month {month} is out of range"),
            Self::DayOutOfRange { day, max } => {
                write!(f, "day {day} is out of range (month has {max} days)")
            }
            Self::YearOutOfRange { year } => write!(f, "year {year} is out of range"),
            Self::JulianYearZero => f.write_str("the Julian calendar has no year 0 (1 BCE is -1)"),
            Self::RataDieOutOfRange { rata_die } => {
                write!(f, "fixed day number {rata_die} is out of range")
            }
            Self::Overflow => f.write_str("date arithmetic overflowed"),
            Self::Parse(p) => write!(f, "parse error: {p}"),
            Self::TimeOutOfRange => f.write_str("time of day is out of range"),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Shape => "expected YYYY-MM-DD",
            Self::NotANumber => "expected an integer field",
            Self::Empty => "empty input",
            Self::Trailing => "trailing characters",
        })
    }
}

impl From<ParseError> for Error {
    fn from(p: ParseError) -> Self {
        Self::Parse(p)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}
