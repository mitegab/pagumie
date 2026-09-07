//! Parsing must never panic on arbitrary bytes, and anything it accepts must
//! round-trip through `Display`.
#![no_main]

use libfuzzer_sys::fuzz_target;

use ethiopic_calendar::{EthiopianDate, GregorianDate, JulianDate};

fuzz_target!(|data: &str| {
    if let Ok(d) = data.parse::<EthiopianDate>() {
        assert_eq!(d.to_string().parse::<EthiopianDate>().unwrap(), d);
        assert_eq!(EthiopianDate::from_rata_die(d.to_rata_die()).unwrap(), d);
    }
    if let Ok(d) = data.parse::<GregorianDate>() {
        assert_eq!(d.to_string().parse::<GregorianDate>().unwrap(), d);
    }
    if let Ok(d) = data.parse::<JulianDate>() {
        assert_ne!(d.year(), 0);
    }
});
