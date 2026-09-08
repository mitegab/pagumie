//! # `libethiopic` — C ABI for the Ethiopian calendar
//!
//! A thin, panic-free, allocation-transparent C interface over
//! [`ethiopic_calendar`], so that programs written in C, C++, Python (`ctypes`),
//! Vala, Zig or anything else with a C FFI can convert dates without
//! reimplementing the arithmetic.
//!
//! ## Contract
//!
//! * Every fallible function returns an [`EthStatus`]; `ETH_STATUS_OK` is 0.
//!   Out-parameters are written only on success.
//! * No function unwinds. The Rust side has no panicking paths on these inputs,
//!   and the release profile is built with `panic = "abort"` as a belt-and-braces
//!   measure.
//! * No function allocates on behalf of the caller. Strings are written into a
//!   caller-supplied buffer, following the `snprintf` convention: the return
//!   value is the length that *would* have been written, so a caller can size a
//!   buffer with a first call that passes `NULL`.
//! * All pointers may be `NULL`; that yields `ETH_STATUS_NULL_POINTER` rather
//!   than undefined behaviour.
//! * The library is thread-safe: it holds no global mutable state.
//!
//! ## Example
//!
//! ```c
//! #include <ethiopic.h>
//! #include <stdio.h>
//!
//! int main(void) {
//!     EthDate d;
//!     if (eth_from_gregorian(2024, 9, 11, &d) != ETH_STATUS_OK) return 1;
//!     printf("%d-%02u-%02u\n", d.year, d.month, d.day);   /* 2017-01-01 */
//!
//!     char buf[128];
//!     eth_format(&d, "%A %-d %B %Y %N", ETH_LOCALE_AMHARIC, buf, sizeof buf);
//!     puts(buf);
//!     return 0;
//! }
//! ```

#![allow(unsafe_code)] // The entire point of this crate.
#![deny(missing_docs)]

use core::ffi::{c_char, c_int};
use std::ffi::CStr;

use ethiopic_calendar::bahire_hasab::{BahireHasab, MovableFeast, ALL_MOVABLE_FEASTS};
use ethiopic_calendar::clock::{DayStart, EthiopianDateTime, EAT_UTC_OFFSET_SECONDS};
use ethiopic_calendar::format::{format_date, format_datetime, Locale};
use ethiopic_calendar::holidays::{holidays_on, Holiday};
use ethiopic_calendar::{
    ethiopic, gregorian, julian, Error, EthiopianDate, GregorianDate, JulianDate, RataDie,
};

/// Status code returned by every fallible entry point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthStatus {
    /// Success.
    Ok = 0,
    /// A required pointer argument was `NULL`.
    NullPointer = 1,
    /// The month index was out of range for the calendar.
    MonthOutOfRange = 2,
    /// The day of the month does not exist in that month of that year.
    DayOutOfRange = 3,
    /// The year cannot be represented.
    YearOutOfRange = 4,
    /// The Julian calendar has no year 0.
    JulianYearZero = 5,
    /// Arithmetic would overflow the supported range.
    Overflow = 6,
    /// A string argument was not valid UTF-8 or was malformed.
    InvalidArgument = 7,
    /// The supplied buffer was too small (the needed size is still returned).
    BufferTooSmall = 8,
    /// The time of day was out of range.
    TimeOutOfRange = 9,
}

impl From<Error> for EthStatus {
    fn from(e: Error) -> Self {
        match e {
            Error::MonthOutOfRange { .. } => Self::MonthOutOfRange,
            Error::DayOutOfRange { .. } => Self::DayOutOfRange,
            Error::YearOutOfRange { .. } | Error::RataDieOutOfRange { .. } => Self::YearOutOfRange,
            Error::JulianYearZero => Self::JulianYearZero,
            Error::Overflow => Self::Overflow,
            Error::TimeOutOfRange => Self::TimeOutOfRange,
            _ => Self::InvalidArgument,
        }
    }
}

/// Which script names are rendered in.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthLocale {
    /// Amharic script.
    Amharic = 0,
    /// Latin script.
    Latin = 1,
}

impl From<EthLocale> for Locale {
    fn from(l: EthLocale) -> Self {
        match l {
            EthLocale::Amharic => Self::Amharic,
            EthLocale::Latin => Self::Latin,
        }
    }
}

/// Which instant begins an Ethiopian calendar day.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthDayStart {
    /// Traditional: the day turns at 06:00 local time.
    Dawn = 0,
    /// Civil: the day turns at midnight.
    Midnight = 1,
}

impl From<EthDayStart> for DayStart {
    fn from(d: EthDayStart) -> Self {
        match d {
            EthDayStart::Dawn => Self::Dawn,
            EthDayStart::Midnight => Self::Midnight,
        }
    }
}

/// A date in the Ethiopian calendar (Amete Mihret).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthDate {
    /// Year in the Amete Mihret era; may be negative (proleptic).
    pub year: i32,
    /// Month, 1–13. Month 13 is Ṗagumēn.
    pub month: u8,
    /// Day of the month, 1–30 (1–5 or 1–6 in Ṗagumēn).
    pub day: u8,
}

/// A date in the Gregorian or Julian calendar.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthWesternDate {
    /// Year; astronomical numbering for Gregorian, no year 0 for Julian.
    pub year: i32,
    /// Month, 1–12.
    pub month: u8,
    /// Day of the month.
    pub day: u8,
}

/// A local Ethiopian date and time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthDateTime {
    /// The date, already adjusted for the chosen day boundary.
    pub date: EthDate,
    /// Ethiopian hour, 1–12.
    pub hour: u8,
    /// Minute, 0–59.
    pub minute: u8,
    /// Second, 0–59.
    pub second: u8,
    /// Western wall-clock hour, 0–23.
    pub wall_hour: u8,
    /// Period of day: 0 ጠዋት, 1 ከሰዓት, 2 ማታ, 3 ለሊት.
    pub period: u8,
    /// Weekday, 0 = Sunday.
    pub weekday: u8,
}

/// Output of the Baḥire Ḥasab computus for one year.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthComputus {
    /// Amete Alem year (`year + 5500`).
    pub amete_alem: i64,
    /// `AA mod 19`.
    pub medeb: u8,
    /// `medeb - 1`, or 18.
    pub wenber: u8,
    /// Solar epact.
    pub abektie: u8,
    /// Lunar epact.
    pub metqi: u8,
    /// Evangelist of the year: 0 Yoḥannes, 1 Matewos, 2 Marqos, 3 Luqas.
    pub evangelist: u8,
    /// Fast of Nineveh.
    pub nineveh: EthDate,
    /// Fasika (Easter).
    pub fasika: EthDate,
}

/// The number of Ethiopian months in a year (thirteen, not twelve).
pub const ETH_MONTHS_IN_YEAR: c_int = 13;

/// UTC offset of East Africa Time, in seconds.
pub const ETH_EAT_UTC_OFFSET_SECONDS: i32 = EAT_UTC_OFFSET_SECONDS;

// ---------------------------------------------------------------- conversions

fn to_eth(d: EthiopianDate) -> EthDate {
    EthDate {
        year: d.year(),
        month: d.month(),
        day: d.day(),
    }
}

// Takes a reference to mirror the `const EthDate *` the C entry points
// hand it, rather than copying at each call site.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn from_eth(d: &EthDate) -> Result<EthiopianDate, EthStatus> {
    EthiopianDate::from_ymd(d.year, d.month, d.day).map_err(EthStatus::from)
}

/// Converts a Gregorian date to the Ethiopian calendar.
///
/// # Safety
/// `out` must be a valid, writable pointer to an [`EthDate`], or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_from_gregorian(
    year: i32,
    month: u8,
    day: u8,
    out: *mut EthDate,
) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    match GregorianDate::from_ymd(year, month, day).and_then(EthiopianDate::from_gregorian) {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Converts an Ethiopian date to the Gregorian calendar.
///
/// # Safety
/// `date` and `out` must be valid pointers, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_to_gregorian(
    date: *const EthDate,
    out: *mut EthWesternDate,
) -> EthStatus {
    if date.is_null() || out.is_null() {
        return EthStatus::NullPointer;
    }
    let d = match from_eth(unsafe { &*date }) {
        Ok(d) => d,
        Err(s) => return s,
    };
    let g = d.to_gregorian();
    unsafe {
        out.write(EthWesternDate {
            year: g.year(),
            month: g.month(),
            day: g.day(),
        });
    }
    EthStatus::Ok
}

/// Converts a Julian-calendar date to the Ethiopian calendar.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_from_julian(
    year: i32,
    month: u8,
    day: u8,
    out: *mut EthDate,
) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    match JulianDate::from_ymd(year, month, day).and_then(EthiopianDate::from_julian) {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Converts an Ethiopian date to the Julian calendar.
///
/// # Safety
/// `date` and `out` must be valid pointers, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_to_julian(
    date: *const EthDate,
    out: *mut EthWesternDate,
) -> EthStatus {
    if date.is_null() || out.is_null() {
        return EthStatus::NullPointer;
    }
    let d = match from_eth(unsafe { &*date }) {
        Ok(d) => d,
        Err(s) => return s,
    };
    match d.to_julian() {
        Ok(j) => {
            unsafe {
                out.write(EthWesternDate {
                    year: j.year(),
                    month: j.month(),
                    day: j.day(),
                });
            }
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Converts an integer Julian Day Number (noon-based) to an Ethiopian date.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_from_jdn(jdn: i64, out: *mut EthDate) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    match EthiopianDate::from_jdn(jdn) {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Returns the Julian Day Number of an Ethiopian date, or `INT64_MIN` if the
/// date is invalid.
///
/// # Safety
/// `date` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_to_jdn(date: *const EthDate) -> i64 {
    if date.is_null() {
        return i64::MIN;
    }
    match from_eth(unsafe { &*date }) {
        Ok(d) => d.to_jdn(),
        Err(_) => i64::MIN,
    }
}

/// Converts a Unix timestamp to a local Ethiopian date and time.
///
/// Pass [`ETH_EAT_UTC_OFFSET_SECONDS`] for Ethiopian local time.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_from_unix(
    timestamp: i64,
    utc_offset_seconds: i32,
    day_start: EthDayStart,
    out: *mut EthDateTime,
) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    match EthiopianDateTime::from_unix_timestamp(timestamp, utc_offset_seconds, day_start.into()) {
        Ok(dt) => {
            let t = dt.time();
            unsafe {
                out.write(EthDateTime {
                    date: to_eth(dt.date()),
                    hour: t.ethiopian_hour(),
                    minute: t.minute(),
                    second: t.second(),
                    wall_hour: t.wall_hour(),
                    period: t.period() as u8,
                    weekday: dt.date().weekday().number(),
                });
            }
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Fills `out` with the current local Ethiopian date and time.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_now(
    utc_offset_seconds: i32,
    day_start: EthDayStart,
    out: *mut EthDateTime,
) -> EthStatus {
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    };
    unsafe { eth_from_unix(now, utc_offset_seconds, day_start, out) }
}

// -------------------------------------------------------------------- queries

/// Whether an Ethiopian year is a leap year (Ṗagumēn has six days).
#[no_mangle]
pub extern "C" fn eth_is_leap_year(year: i32) -> bool {
    ethiopic::is_leap_year(year)
}

/// Days in an Ethiopian month; 0 if the month index is invalid.
#[no_mangle]
pub extern "C" fn eth_days_in_month(year: i32, month: u8) -> u8 {
    ethiopic::days_in_month(year, month)
}

/// Days in an Ethiopian year: 365 or 366.
#[no_mangle]
pub extern "C" fn eth_days_in_year(year: i32) -> u16 {
    ethiopic::days_in_year(year)
}

/// Whether a Gregorian year is a leap year.
#[no_mangle]
pub extern "C" fn eth_gregorian_is_leap_year(year: i32) -> bool {
    gregorian::is_leap_year(year)
}

/// Whether a Julian year is a leap year.
#[no_mangle]
pub extern "C" fn eth_julian_is_leap_year(year: i32) -> bool {
    julian::is_leap_year(year)
}

/// Day of the week of an Ethiopian date, 0 = Sunday; 255 if the date is invalid.
///
/// # Safety
/// `date` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_weekday(date: *const EthDate) -> u8 {
    if date.is_null() {
        return u8::MAX;
    }
    match from_eth(unsafe { &*date }) {
        Ok(d) => d.weekday().number(),
        Err(_) => u8::MAX,
    }
}

/// Adds a signed number of days to an Ethiopian date.
///
/// # Safety
/// `date` and `out` must be valid pointers, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_add_days(
    date: *const EthDate,
    days: i64,
    out: *mut EthDate,
) -> EthStatus {
    if date.is_null() || out.is_null() {
        return EthStatus::NullPointer;
    }
    let d = match from_eth(unsafe { &*date }) {
        Ok(d) => d,
        Err(s) => return s,
    };
    match d.add_days(days) {
        Ok(r) => {
            unsafe { out.write(to_eth(r)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Number of days from `from` to `to`; `INT64_MIN` if either date is invalid.
///
/// # Safety
/// Both pointers must be valid, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_days_between(from: *const EthDate, to: *const EthDate) -> i64 {
    if from.is_null() || to.is_null() {
        return i64::MIN;
    }
    let (Ok(a), Ok(b)) = (from_eth(unsafe { &*from }), from_eth(unsafe { &*to })) else {
        return i64::MIN;
    };
    b.days_since(a)
}

// ------------------------------------------------------------------- computus

/// Runs the Baḥire Ḥasab computus for an Ethiopian year.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_computus(year: i32, out: *mut EthComputus) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    let bh = match BahireHasab::new(year) {
        Ok(b) => b,
        Err(e) => return e.into(),
    };
    let (Ok(nineveh), Ok(fasika)) = (
        bh.feast(MovableFeast::Nineveh),
        bh.feast(MovableFeast::Tinsae),
    ) else {
        return EthStatus::Overflow;
    };
    unsafe {
        out.write(EthComputus {
            amete_alem: bh.amete_alem(),
            medeb: bh.medeb(),
            wenber: bh.wenber(),
            abektie: bh.abektie(),
            metqi: bh.metqi(),
            evangelist: bh.evangelist() as u8,
            nineveh: to_eth(nineveh),
            fasika: to_eth(fasika),
        });
    }
    EthStatus::Ok
}

/// Number of movable feasts, i.e. the valid range of `index` for
/// [`eth_movable_feast`].
pub const ETH_MOVABLE_FEAST_COUNT: c_int = 11;

/// Writes the date of the `index`-th movable feast of `year`, in chronological
/// order (0 = Fast of Nineveh, 5 = Fasika).
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_movable_feast(
    year: i32,
    index: c_int,
    out: *mut EthDate,
) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    let Ok(idx) = usize::try_from(index) else {
        return EthStatus::InvalidArgument;
    };
    let Some(&feast) = ALL_MOVABLE_FEASTS.get(idx) else {
        return EthStatus::InvalidArgument;
    };
    match BahireHasab::new(year).and_then(|bh| bh.feast(feast)) {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Name of the `index`-th movable feast, as a static NUL-terminated string, or
/// `NULL` if `index` is out of range. The pointer is valid for the lifetime of
/// the process and must not be freed.
#[no_mangle]
pub extern "C" fn eth_movable_feast_name(index: c_int, locale: EthLocale) -> *const c_char {
    let Ok(idx) = usize::try_from(index) else {
        return core::ptr::null();
    };
    let Some(feast) = ALL_MOVABLE_FEASTS.get(idx) else {
        return core::ptr::null();
    };
    static_cstr(match locale {
        EthLocale::Amharic => feast.amharic(),
        EthLocale::Latin => feast.english(),
    })
}

/// Name of the holiday falling on `date`, or `NULL` if it is an ordinary day.
///
/// The returned pointer is static and must not be freed.
///
/// # Safety
/// `date` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_holiday_name(
    date: *const EthDate,
    locale: EthLocale,
) -> *const c_char {
    if date.is_null() {
        return core::ptr::null();
    }
    let Ok(d) = from_eth(unsafe { &*date }) else {
        return core::ptr::null();
    };
    let Some(h) = holidays_on(d).next() else {
        return core::ptr::null();
    };
    static_cstr(match locale {
        EthLocale::Amharic => h.amharic(),
        EthLocale::Latin => h.english(),
    })
}

/// Whether `date` is a public holiday.
///
/// # Safety
/// `date` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_is_holiday(date: *const EthDate) -> bool {
    if date.is_null() {
        return false;
    }
    from_eth(unsafe { &*date }).is_ok_and(ethiopic_calendar::holidays::is_holiday)
}

// ------------------------------------------------------------------ formatting

/// Formats a date into `buf` using the `strftime`-style grammar documented in
/// `ethiopic_calendar::format`.
///
/// Follows the `snprintf` convention: returns the number of bytes the complete
/// output would occupy, excluding the terminating NUL. If that is greater than
/// or equal to `len`, the output was truncated; call again with a larger buffer.
/// Passing `NULL`/0 for the buffer is allowed and returns the required size.
/// Returns a negative value on error.
///
/// # Safety
/// `date` must be valid; `fmt` must be a NUL-terminated C string; `buf` must be
/// writable for `len` bytes or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_format(
    date: *const EthDate,
    fmt: *const c_char,
    locale: EthLocale,
    buf: *mut c_char,
    len: usize,
) -> isize {
    if date.is_null() || fmt.is_null() {
        return -(EthStatus::NullPointer as isize);
    }
    let Ok(d) = from_eth(unsafe { &*date }) else {
        return -(EthStatus::DayOutOfRange as isize);
    };
    let Ok(fmt) = unsafe { CStr::from_ptr(fmt) }.to_str() else {
        return -(EthStatus::InvalidArgument as isize);
    };
    let s = format_date(d, fmt, locale.into());
    unsafe { write_out(&s, buf, len) }
}

/// Formats a date *and time* into `buf`. See [`eth_format`] for the convention.
///
/// # Safety
/// As [`eth_format`], plus `dt` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn eth_format_datetime(
    timestamp: i64,
    utc_offset_seconds: i32,
    day_start: EthDayStart,
    fmt: *const c_char,
    locale: EthLocale,
    buf: *mut c_char,
    len: usize,
) -> isize {
    if fmt.is_null() {
        return -(EthStatus::NullPointer as isize);
    }
    let Ok(fmt) = unsafe { CStr::from_ptr(fmt) }.to_str() else {
        return -(EthStatus::InvalidArgument as isize);
    };
    let Ok(dt) =
        EthiopianDateTime::from_unix_timestamp(timestamp, utc_offset_seconds, day_start.into())
    else {
        return -(EthStatus::Overflow as isize);
    };
    let s = format_datetime(&dt, fmt, locale.into());
    unsafe { write_out(&s, buf, len) }
}

/// Parses `YYYY-MM-DD` as an Ethiopian date.
///
/// # Safety
/// `s` must be a NUL-terminated C string; `out` must be writable or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_parse(s: *const c_char, out: *mut EthDate) -> EthStatus {
    if s.is_null() || out.is_null() {
        return EthStatus::NullPointer;
    }
    let Ok(s) = unsafe { CStr::from_ptr(s) }.to_str() else {
        return EthStatus::InvalidArgument;
    };
    match s.parse::<EthiopianDate>() {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Static, NUL-terminated month name; `NULL` if `month` is not 1–13.
#[no_mangle]
pub extern "C" fn eth_month_name(month: u8, locale: EthLocale) -> *const c_char {
    let Ok(m) = ethiopic::Month::from_number(month) else {
        return core::ptr::null();
    };
    static_cstr(match locale {
        EthLocale::Amharic => m.amharic(),
        EthLocale::Latin => m.latin(),
    })
}

/// Static, NUL-terminated weekday name; `NULL` if `weekday` is not 0–6.
#[no_mangle]
pub extern "C" fn eth_weekday_name(weekday: u8, locale: EthLocale) -> *const c_char {
    let Some(w) = ethiopic_calendar::weekday::ALL_WEEKDAYS.get(weekday as usize) else {
        return core::ptr::null();
    };
    static_cstr(match locale {
        EthLocale::Amharic => w.amharic(),
        EthLocale::Latin => w.english(),
    })
}

/// Human-readable description of a status code. Never `NULL`.
#[no_mangle]
pub extern "C" fn eth_strerror(status: EthStatus) -> *const c_char {
    static_cstr(match status {
        EthStatus::Ok => "success",
        EthStatus::NullPointer => "null pointer argument",
        EthStatus::MonthOutOfRange => "month out of range",
        EthStatus::DayOutOfRange => "day out of range for that month",
        EthStatus::YearOutOfRange => "year out of range",
        EthStatus::JulianYearZero => "the Julian calendar has no year 0",
        EthStatus::Overflow => "date arithmetic overflowed",
        EthStatus::InvalidArgument => "invalid argument",
        EthStatus::BufferTooSmall => "buffer too small",
        EthStatus::TimeOutOfRange => "time of day out of range",
    })
}

/// Library version as a static, NUL-terminated string, e.g. `"0.1.0"`.
#[no_mangle]
pub extern "C" fn eth_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

// ------------------------------------------------------------------- internals

/// Returns a pointer to a NUL-terminated copy of a `&'static str`.
///
/// The name tables in `ethiopic_calendar` are `&'static str` and therefore not
/// NUL-terminated. Rather than allocate (which would force the caller to free),
/// this leaks one small `CString` per distinct string, at most a few dozen for
/// the lifetime of the process — the same trade-off `strerror` makes.
fn static_cstr(s: &'static str) -> *const c_char {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<&'static str, usize>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cache.lock() else {
        return core::ptr::null();
    };
    let ptr = *map.entry(s).or_insert_with(|| {
        let owned = std::ffi::CString::new(s).unwrap_or_default();
        std::ffi::CString::into_raw(owned) as usize
    });
    ptr as *const c_char
}

/// `snprintf`-style write of a Rust string into a caller buffer.
unsafe fn write_out(s: &str, buf: *mut c_char, len: usize) -> isize {
    let needed = s.len();
    if buf.is_null() || len == 0 {
        return needed as isize;
    }
    let copy = needed.min(len - 1);
    // Never split a UTF-8 sequence: back off to a character boundary.
    let copy = (0..=copy)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr().cast::<c_char>(), buf, copy);
        buf.add(copy).write(0);
    }
    needed as isize
}

/// Converts an R.D. day number to an Ethiopian date; exposed because status-bar
/// modules often already have a day number in hand.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_from_rata_die(rata_die: i64, out: *mut EthDate) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    match EthiopianDate::from_rata_die(RataDie::new(rata_die)) {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

/// Name of a holiday by its stable key index, for enumeration; `NULL` past the
/// end. Indices are stable within a major version.
#[no_mangle]
pub extern "C" fn eth_holiday_key(index: c_int) -> *const c_char {
    let Ok(idx) = usize::try_from(index) else {
        return core::ptr::null();
    };
    match ethiopic_calendar::holidays::ALL_HOLIDAYS.get(idx) {
        Some(h) => static_cstr(h.key()),
        None => core::ptr::null(),
    }
}

/// Date of the holiday with the given key index in `year`.
///
/// # Safety
/// `out` must be a valid pointer, or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn eth_holiday_date(
    index: c_int,
    year: i32,
    observed: bool,
    out: *mut EthDate,
) -> EthStatus {
    if out.is_null() {
        return EthStatus::NullPointer;
    }
    let Ok(idx) = usize::try_from(index) else {
        return EthStatus::InvalidArgument;
    };
    let Some(&h) = ethiopic_calendar::holidays::ALL_HOLIDAYS.get(idx) else {
        return EthStatus::InvalidArgument;
    };
    let result = if observed {
        Holiday::observed_date(h, year)
    } else {
        Holiday::date(h, year)
    };
    match result {
        Ok(d) => {
            unsafe { out.write(to_eth(d)) };
            EthStatus::Ok
        }
        Err(e) => e.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn gregorian_roundtrip_through_the_abi() {
        let mut d = EthDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(
            unsafe { eth_from_gregorian(2024, 9, 11, &mut d) },
            EthStatus::Ok
        );
        assert_eq!((d.year, d.month, d.day), (2017, 1, 1));
        let mut g = EthWesternDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(unsafe { eth_to_gregorian(&d, &mut g) }, EthStatus::Ok);
        assert_eq!((g.year, g.month, g.day), (2024, 9, 11));
    }

    #[test]
    fn null_pointers_are_errors_not_crashes() {
        assert_eq!(
            unsafe { eth_from_gregorian(2024, 9, 11, core::ptr::null_mut()) },
            EthStatus::NullPointer
        );
        assert_eq!(
            unsafe { eth_to_gregorian(core::ptr::null(), core::ptr::null_mut()) },
            EthStatus::NullPointer
        );
        assert_eq!(unsafe { eth_to_jdn(core::ptr::null()) }, i64::MIN);
        assert_eq!(unsafe { eth_weekday(core::ptr::null()) }, u8::MAX);
        assert!(unsafe { eth_holiday_name(core::ptr::null(), EthLocale::Latin) }.is_null());
    }

    #[test]
    fn invalid_dates_are_rejected() {
        let mut d = EthDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(
            unsafe { eth_from_gregorian(2023, 2, 29, &mut d) },
            EthStatus::DayOutOfRange
        );
        assert_eq!(
            unsafe { eth_from_gregorian(2023, 13, 1, &mut d) },
            EthStatus::MonthOutOfRange
        );
        let bad = EthDate {
            year: 2016,
            month: 13,
            day: 6,
        };
        let mut g = EthWesternDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(
            unsafe { eth_to_gregorian(&bad, &mut g) },
            EthStatus::DayOutOfRange
        );
    }

    #[test]
    fn format_follows_the_snprintf_convention() {
        let d = EthDate {
            year: 2018,
            month: 1,
            day: 1,
        };
        let fmt = CString::new("%Y-%m-%d").unwrap();
        let needed =
            unsafe { eth_format(&d, fmt.as_ptr(), EthLocale::Latin, core::ptr::null_mut(), 0) };
        assert_eq!(needed, 10);
        let mut buf = [0i8; 32];
        let n = unsafe {
            eth_format(
                &d,
                fmt.as_ptr(),
                EthLocale::Latin,
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        assert_eq!(n, 10);
        let s = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_str().unwrap();
        assert_eq!(s, "2018-01-01");

        // Truncation reports the full length and never splits a UTF-8 sequence.
        let fmt = CString::new("%B").unwrap();
        let mut small = [0i8; 5];
        let n = unsafe {
            eth_format(
                &d,
                fmt.as_ptr(),
                EthLocale::Amharic,
                small.as_mut_ptr(),
                small.len(),
            )
        };
        assert!(n > 5);
        let s = unsafe { CStr::from_ptr(small.as_ptr()) }.to_str();
        assert!(s.is_ok(), "truncated output must stay valid UTF-8");
    }

    #[test]
    fn computus_through_the_abi() {
        let mut c = EthComputus {
            amete_alem: 0,
            medeb: 0,
            wenber: 0,
            abektie: 0,
            metqi: 0,
            evangelist: 0,
            nineveh: EthDate {
                year: 0,
                month: 0,
                day: 0,
            },
            fasika: EthDate {
                year: 0,
                month: 0,
                day: 0,
            },
        };
        assert_eq!(unsafe { eth_computus(2017, &mut c) }, EthStatus::Ok);
        assert_eq!(c.amete_alem, 7517);
        let mut g = EthWesternDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(
            unsafe { eth_to_gregorian(&c.fasika, &mut g) },
            EthStatus::Ok
        );
        assert_eq!((g.year, g.month, g.day), (2025, 4, 20));
    }

    #[test]
    fn names_and_errors_are_stable_pointers() {
        let a = eth_month_name(13, EthLocale::Latin);
        let b = eth_month_name(13, EthLocale::Latin);
        assert!(!a.is_null());
        assert_eq!(a, b, "repeated calls must not leak a new string each time");
        assert_eq!(unsafe { CStr::from_ptr(a) }.to_str().unwrap(), "Pagume");
        assert!(eth_month_name(14, EthLocale::Latin).is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(eth_strerror(EthStatus::JulianYearZero)) }
                .to_str()
                .unwrap(),
            "the Julian calendar has no year 0"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(eth_version()) }.to_str().unwrap(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn parse_and_arithmetic() {
        let s = CString::new("2015-13-06").unwrap();
        let mut d = EthDate {
            year: 0,
            month: 0,
            day: 0,
        };
        assert_eq!(unsafe { eth_parse(s.as_ptr(), &mut d) }, EthStatus::Ok);
        assert_eq!((d.year, d.month, d.day), (2015, 13, 6));
        let mut next = d;
        assert_eq!(unsafe { eth_add_days(&d, 1, &mut next) }, EthStatus::Ok);
        assert_eq!((next.year, next.month, next.day), (2016, 1, 1));
        assert_eq!(unsafe { eth_days_between(&d, &next) }, 1);
        assert!(eth_is_leap_year(2015));
        assert_eq!(eth_days_in_month(2015, 13), 6);
        assert_eq!(eth_days_in_month(2016, 13), 5);
    }
}
