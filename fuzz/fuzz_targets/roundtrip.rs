//! Round-trip every calendar through a fuzzer-chosen fixed day number.
//!
//! The invariant is absolute: for *any* R.D. in range, converting to a calendar
//! and back must be the identity, and the produced date must be structurally
//! valid. A crash or a mismatch here is a real bug, not a heuristic finding.
#![no_main]

use libfuzzer_sys::fuzz_target;

use ethiopic_calendar::ethiopic::days_in_month;
use ethiopic_calendar::{EthiopianDate, GregorianDate, JulianDate, RataDie};

fuzz_target!(|input: i64| {
    // Keep inside the documented range; outside it the API returns Err, which
    // is also exercised below.
    let rd = RataDie::new(input % 800_000_000);

    let e = EthiopianDate::from_rata_die(rd).expect("in-range R.D. must convert");
    assert_eq!(e.to_rata_die(), rd);
    assert!((1..=13).contains(&e.month()));
    assert!(e.day() >= 1 && e.day() <= days_in_month(e.year(), e.month()));

    let g = GregorianDate::from_rata_die(rd).expect("in-range R.D. must convert");
    assert_eq!(g.to_rata_die(), rd);
    assert_eq!(e.to_gregorian(), g);
    assert_eq!(EthiopianDate::from_gregorian(g).unwrap(), e);

    let j = JulianDate::from_rata_die(rd).expect("in-range R.D. must convert");
    assert_eq!(j.to_rata_die(), rd);
    assert_ne!(j.year(), 0);

    assert_eq!(e.weekday(), g.weekday());
    assert_eq!(g.weekday(), j.weekday());

    // Out-of-range input must error rather than panic or wrap.
    let far = RataDie::new(i64::MAX / 2);
    assert!(EthiopianDate::from_rata_die(far).is_err());
});
