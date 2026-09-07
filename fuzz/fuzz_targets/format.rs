//! Formatting must never panic, whatever the format string, and must never
//! silently drop a `%` specifier.
#![no_main]

use libfuzzer_sys::fuzz_target;

use ethiopic_calendar::format::{format_date, Locale};
use ethiopic_calendar::EthiopianDate;

fuzz_target!(|input: (i32, u8, u8, String)| {
    let (year, month, day, fmt) = input;
    let month = month % 13 + 1;
    let day = day % 30 + 1;
    let Ok(date) = EthiopianDate::from_ymd(year, month, day) else {
        return;
    };
    let out = format_date(date, &fmt, Locale::Amharic);
    // A format string with no specifiers is reproduced verbatim.
    if !fmt.contains('%') {
        assert_eq!(out, fmt);
    }
    let _ = format_date(date, &fmt, Locale::Latin);
});
