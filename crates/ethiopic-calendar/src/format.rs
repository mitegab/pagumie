//! `strftime`-style formatting for Ethiopian dates and times.
//!
//! The conversion specifiers follow POSIX `strftime(3)` wherever a POSIX
//! specifier has an unambiguous Ethiopian meaning, and add a small set of
//! Ethiopic-specific ones. This is the same grammar the `ethdate` CLI, the
//! C ABI (`eth_format`) and the status-bar modules use, so a format string
//! learned in one place works in all of them.
//!
//! | Spec | Meaning | Example (Meskerem 1, 2018, 09:30) |
//! |------|---------|-----------------------------------|
//! | `%Y` | Year, zero-padded to 4 | `2018` |
//! | `%y` | Year mod 100, zero-padded | `18` |
//! | `%C` | Century | `20` |
//! | `%G` | Year in Ge'ez numerals | `፳፻፲፰` |
//! | `%m` | Month number, zero-padded | `01` |
//! | `%-m` | Month number, unpadded | `1` |
//! | `%d` | Day of month, zero-padded | `01` |
//! | `%-d` | Day of month, unpadded | `1` |
//! | `%e` | Day of month, space-padded | ` 1` |
//! | `%B` | Month name, Amharic | `መስከረም` |
//! | `%b` | Month name, Latin | `Meskerem` |
//! | `%h` | Month name, 3-letter Latin | `Msk` |
//! | `%A` | Weekday, Amharic | `ሐሙስ` |
//! | `%a` | Weekday, 3-letter English | `Thu` |
//! | `%q` | Weekday, Latin transliteration | `Hamus` |
//! | `%u` | Weekday, ISO number (Monday = 1) | `4` |
//! | `%w` | Weekday number (Sunday = 0) | `4` |
//! | `%j` | Day of year, zero-padded to 3 | `001` |
//! | `%E` | Era, Latin abbreviation | `E.C.` |
//! | `%N` | Era, Amharic abbreviation | `ዓ.ም.` |
//! | `%L` | Year in the Amete Alem era | `7518` |
//! | `%F` | `%Y-%m-%d` | `2018-01-01` |
//! | `%D` | `%m/%d/%y` | `01/01/18` |
//! | `%s` | Unix timestamp of midnight UTC | `1757548800` |
//! | `%J` | Julian Day Number | `2460930` |
//! | `%R` | Fixed day number (R.D.) | `739505` |
//! | `%g` | Gregorian equivalent, ISO | `2025-09-11` |
//! | `%I` | Ethiopian hour, `1..=12` | `3` |
//! | `%K` | Ethiopian hour, zero-padded | `03` |
//! | `%H` | Western hour, `00..=23` | `09` |
//! | `%M` | Minute | `30` |
//! | `%S` | Second | `00` |
//! | `%p` | Period, Amharic | `ጠዋት` |
//! | `%P` | Period, romanised | `tewat` |
//! | `%T` | `%I:%M:%S %p` | `3:30:00 ጠዋት` |
//! | `%Z` | Time-zone abbreviation | `EAT` |
//! | `%z` | UTC offset | `+0300` |
//! | `%n`, `%t`, `%%` | newline, tab, literal `%` | |
//!
//! Unknown specifiers are reproduced verbatim (`%Q` formats as `%Q`) rather
//! than being dropped, so a typo is visible instead of silent.

use alloc::string::String;
use core::fmt::Write as _;

use crate::clock::{EthiopianDateTime, EthiopianTime};
use crate::era::Era;
use crate::ethiopic::EthiopianDate;
use crate::names::geez_numeral;

/// Which script to use for names that have both an Amharic and a Latin form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Locale {
    /// Amharic script for `%B`/`%A`, Amharic era marker.
    #[default]
    Amharic,
    /// Latin script.
    Latin,
}

/// Formats an Ethiopian date, without any time-of-day component.
///
/// Time specifiers render as if midnight; use [`format_datetime`] to supply a
/// real clock reading.
///
/// ```
/// use ethiopic_calendar::{EthiopianDate, format::{format_date, Locale}};
///
/// let d = EthiopianDate::from_ymd(2018, 1, 1).unwrap();
/// assert_eq!(format_date(d, "%Y-%m-%d", Locale::Amharic), "2018-01-01");
/// assert_eq!(format_date(d, "%d %B %Y %N", Locale::Amharic), "01 መስከረም 2018 ዓ.ም.");
/// assert_eq!(format_date(d, "%-d %b %Y %E", Locale::Latin), "1 Meskerem 2018 E.C.");
/// assert_eq!(format_date(d, "%G", Locale::Amharic), "፳፻፲፰");
/// ```
#[must_use]
pub fn format_date(date: EthiopianDate, fmt: &str, locale: Locale) -> String {
    let time = EthiopianTime::default();
    format_parts(date, time, None, fmt, locale)
}

/// Formats a full Ethiopian date and time.
#[must_use]
pub fn format_datetime(dt: &EthiopianDateTime, fmt: &str, locale: Locale) -> String {
    format_parts(
        dt.date(),
        dt.time(),
        Some(dt.utc_offset_seconds()),
        fmt,
        locale,
    )
}

#[allow(clippy::too_many_lines)]
fn format_parts(
    date: EthiopianDate,
    time: EthiopianTime,
    utc_offset: Option<i32>,
    fmt: &str,
    locale: Locale,
) -> String {
    let mut out = String::with_capacity(fmt.len() + 16);
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // `%-X` suppresses zero padding, as in glibc.
        let no_pad = chars.peek() == Some(&'-');
        if no_pad {
            chars.next();
        }
        let Some(spec) = chars.next() else {
            out.push('%');
            break;
        };
        match spec {
            'Y' => pad_year(&mut out, date.year(), no_pad),
            'y' => {
                let _ = write!(out, "{:02}", date.year().rem_euclid(100));
            }
            'C' => {
                let _ = write!(out, "{}", date.year().div_euclid(100));
            }
            'G' => out.push_str(&geez_numeral(i64::from(date.year()))),
            'm' => num(&mut out, u32::from(date.month()), 2, no_pad),
            'd' => num(&mut out, u32::from(date.day()), 2, no_pad),
            'e' => {
                let _ = write!(out, "{:2}", date.day());
            }
            'B' => out.push_str(match locale {
                Locale::Amharic => date.month_enum().amharic(),
                Locale::Latin => date.month_enum().latin(),
            }),
            'b' => out.push_str(date.month_enum().latin()),
            'h' => out.push_str(date.month_enum().abbrev()),
            'A' => out.push_str(match locale {
                Locale::Amharic => date.weekday().amharic(),
                Locale::Latin => date.weekday().english(),
            }),
            'a' => out.push_str(date.weekday().english_abbrev()),
            'q' => out.push_str(date.weekday().transliterated()),
            'u' => {
                let _ = write!(out, "{}", date.weekday().iso_number());
            }
            'w' => {
                let _ = write!(out, "{}", date.weekday().number());
            }
            'j' => num(&mut out, u32::from(date.day_of_year()), 3, no_pad),
            'E' => out.push_str(Era::AmeteMihret.latin_abbrev()),
            'N' => out.push_str(Era::AmeteMihret.amharic_abbrev()),
            'L' => {
                let _ = write!(out, "{}", date.amete_alem());
            }
            'F' => {
                pad_year(&mut out, date.year(), false);
                let _ = write!(out, "-{:02}-{:02}", date.month(), date.day());
            }
            'D' => {
                let _ = write!(
                    out,
                    "{:02}/{:02}/{:02}",
                    date.month(),
                    date.day(),
                    date.year().rem_euclid(100)
                );
            }
            's' => {
                let _ = write!(
                    out,
                    "{}",
                    date.to_rata_die().to_unix_timestamp()
                        + i64::from(time.seconds_since_midnight())
                        - i64::from(utc_offset.unwrap_or(0))
                );
            }
            'J' => {
                let _ = write!(out, "{}", date.to_jdn());
            }
            'R' => {
                let _ = write!(out, "{}", date.to_rata_die().get());
            }
            'g' => {
                let g = date.to_gregorian();
                let _ = write!(out, "{g}");
            }
            'I' => {
                let _ = write!(out, "{}", time.ethiopian_hour());
            }
            'K' => {
                let _ = write!(out, "{:02}", time.ethiopian_hour());
            }
            'H' => num(&mut out, u32::from(time.wall_hour()), 2, no_pad),
            'M' => num(&mut out, u32::from(time.minute()), 2, no_pad),
            'S' => num(&mut out, u32::from(time.second()), 2, no_pad),
            'p' => out.push_str(match locale {
                Locale::Amharic => time.period().amharic(),
                Locale::Latin => time.period().english(),
            }),
            'P' => out.push_str(time.period().transliterated()),
            'T' => {
                let _ = write!(
                    out,
                    "{}:{:02}:{:02} {}",
                    time.ethiopian_hour(),
                    time.minute(),
                    time.second(),
                    match locale {
                        Locale::Amharic => time.period().amharic(),
                        Locale::Latin => time.period().english(),
                    }
                );
            }
            'Z' => out.push_str(zone_abbrev(utc_offset)),
            'z' => {
                let off = utc_offset.unwrap_or(0);
                let sign = if off < 0 { '-' } else { '+' };
                let a = off.abs();
                let _ = write!(out, "{sign}{:02}{:02}", a / 3600, (a % 3600) / 60);
            }
            'n' => out.push('\n'),
            't' => out.push('\t'),
            '%' => out.push('%'),
            other => {
                // Preserve unknown specifiers verbatim, including the `-`.
                out.push('%');
                if no_pad {
                    out.push('-');
                }
                out.push(other);
            }
        }
    }
    out
}

fn zone_abbrev(utc_offset: Option<i32>) -> &'static str {
    match utc_offset {
        Some(o) if o == crate::clock::EAT_UTC_OFFSET_SECONDS => "EAT",
        Some(0) => "UTC",
        Some(_) | None => "",
    }
}

fn pad_year(out: &mut String, year: i32, no_pad: bool) {
    if no_pad {
        let _ = write!(out, "{year}");
    } else if year < 0 {
        let _ = write!(out, "-{:04}", i64::from(year).abs());
    } else {
        let _ = write!(out, "{year:04}");
    }
}

fn num(out: &mut String, value: u32, width: usize, no_pad: bool) {
    if no_pad {
        let _ = write!(out, "{value}");
    } else {
        let _ = write!(out, "{value:0width$}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{DayStart, Period, EAT_UTC_OFFSET_SECONDS};
    use alloc::string::ToString;

    fn d() -> EthiopianDate {
        EthiopianDate::from_ymd(2018, 1, 1).unwrap()
    }

    #[test]
    fn date_specifiers() {
        assert_eq!(format_date(d(), "%Y-%m-%d", Locale::Amharic), "2018-01-01");
        assert_eq!(format_date(d(), "%F", Locale::Amharic), "2018-01-01");
        assert_eq!(format_date(d(), "%y %C", Locale::Amharic), "18 20");
        assert_eq!(format_date(d(), "%-m/%-d", Locale::Amharic), "1/1");
        assert_eq!(format_date(d(), "%B", Locale::Amharic), "መስከረም");
        assert_eq!(format_date(d(), "%B", Locale::Latin), "Meskerem");
        assert_eq!(format_date(d(), "%h", Locale::Latin), "Msk");
        assert_eq!(format_date(d(), "%j", Locale::Amharic), "001");
        assert_eq!(format_date(d(), "%L %E", Locale::Amharic), "7518 E.C.");
        assert_eq!(format_date(d(), "%g", Locale::Amharic), "2025-09-11");
        assert_eq!(format_date(d(), "%G", Locale::Amharic), "፳፻፲፰");
    }

    #[test]
    fn weekday_specifiers() {
        // Meskerem 1, 2018 = Thursday 11 September 2025.
        assert_eq!(format_date(d(), "%A", Locale::Amharic), "ሐሙስ");
        assert_eq!(format_date(d(), "%A", Locale::Latin), "Thursday");
        assert_eq!(
            format_date(d(), "%a %q %u %w", Locale::Latin),
            "Thu Hamus 4 4"
        );
    }

    #[test]
    fn epagomenal_and_negative_years() {
        let p = EthiopianDate::from_ymd(2015, 13, 6).unwrap();
        assert_eq!(format_date(p, "%d %B %Y", Locale::Amharic), "06 ጳጉሜን 2015");
        assert_eq!(format_date(p, "%j", Locale::Amharic), "366");
        let old = EthiopianDate::from_ymd(-44, 5, 3).unwrap();
        assert_eq!(format_date(old, "%Y-%m-%d", Locale::Amharic), "-0044-05-03");
        assert_eq!(format_date(old, "%-Y", Locale::Amharic), "-44");
    }

    #[test]
    fn literals_and_unknown_specifiers() {
        assert_eq!(format_date(d(), "100%%", Locale::Amharic), "100%");
        assert_eq!(format_date(d(), "%Q", Locale::Amharic), "%Q");
        assert_eq!(format_date(d(), "%-Q", Locale::Amharic), "%-Q");
        assert_eq!(format_date(d(), "a%nb%tc", Locale::Amharic), "a\nb\tc");
        assert_eq!(
            format_date(d(), "trailing %", Locale::Amharic),
            "trailing %"
        );
        assert_eq!(format_date(d(), "", Locale::Amharic), "");
    }

    #[test]
    fn time_specifiers() {
        let dt = EthiopianDateTime::new(
            d(),
            EthiopianTime::from_wall_clock(9, 30, 5).unwrap(),
            EAT_UTC_OFFSET_SECONDS,
            DayStart::Dawn,
        );
        assert_eq!(
            format_datetime(&dt, "%H:%M:%S", Locale::Amharic),
            "09:30:05"
        );
        assert_eq!(
            format_datetime(&dt, "%I:%M %p", Locale::Amharic),
            "3:30 ጠዋት"
        );
        assert_eq!(format_datetime(&dt, "%K", Locale::Amharic), "03");
        assert_eq!(format_datetime(&dt, "%P", Locale::Amharic), "tewat");
        assert_eq!(format_datetime(&dt, "%Z %z", Locale::Amharic), "EAT +0300");
        assert_eq!(format_datetime(&dt, "%T", Locale::Amharic), "3:30:05 ጠዋት");
        assert_eq!(dt.time().period(), Period::Tewat);
    }

    #[test]
    fn unix_and_day_number_specifiers() {
        let s = format_date(d(), "%J %R %s", Locale::Amharic);
        let parts: alloc::vec::Vec<&str> = s.split(' ').collect();
        assert_eq!(parts[0].parse::<i64>().unwrap(), d().to_jdn());
        assert_eq!(parts[1].parse::<i64>().unwrap(), d().to_rata_die().get());
        assert_eq!(
            parts[2].parse::<i64>().unwrap(),
            d().to_rata_die().to_unix_timestamp()
        );
        assert_eq!(d().to_string(), "2018-01-01");
    }
}
