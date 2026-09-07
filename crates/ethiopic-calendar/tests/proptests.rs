//! Property-based tests over the full supported date range.
//!
//! Unit tests pin down the anchors; these check the *identities* that must hold
//! everywhere. Ranges deliberately reach far outside the historically
//! meaningful window — a proleptic calendar has no excuse for breaking at year
//! −2 000 000 — and include year 0, the epoch neighbourhoods, and the
//! epagomenal month.

use proptest::prelude::*;

use ethiopic_calendar::bahire_hasab::{BahireHasab, MovableFeast};
use ethiopic_calendar::clock::{DayStart, EthiopianDateTime, EthiopianTime, Period};
use ethiopic_calendar::ethiopic::{days_in_month, days_in_year};
use ethiopic_calendar::{EthiopianDate, GregorianDate, JulianDate, RataDie, Weekday};

/// Wide enough to exercise every branch, small enough that `i32` years and the
/// `i64` intermediates are both far from their limits.
const RD_RANGE: std::ops::RangeInclusive<i64> = -800_000_000..=800_000_000;

/// A tighter range for tests that need a *valid Julian* year (no year 0).
const CIVIL_RD_RANGE: std::ops::RangeInclusive<i64> = -2_000_000..=2_000_000;

prop_compose! {
    fn any_ethiopian_date()(
        year in -3_000_000i32..3_000_000,
        month in 1u8..=13,
        day in 1u8..=30,
    ) -> EthiopianDate {
        let day = day.min(days_in_month(year, month));
        EthiopianDate::from_ymd(year, month, day).expect("clamped day is always valid")
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// R.D. → Ethiopic → R.D. is the identity everywhere.
    #[test]
    fn ethiopic_roundtrips_through_rata_die(rd in RD_RANGE) {
        let rd = RataDie::new(rd);
        let d = EthiopianDate::from_rata_die(rd).unwrap();
        prop_assert_eq!(d.to_rata_die(), rd);
    }

    #[test]
    fn gregorian_roundtrips_through_rata_die(rd in RD_RANGE) {
        let rd = RataDie::new(rd);
        let d = GregorianDate::from_rata_die(rd).unwrap();
        prop_assert_eq!(d.to_rata_die(), rd);
    }

    #[test]
    fn julian_roundtrips_through_rata_die(rd in RD_RANGE) {
        let rd = RataDie::new(rd);
        let d = JulianDate::from_rata_die(rd).unwrap();
        prop_assert_eq!(d.to_rata_die(), rd);
        prop_assert_ne!(d.year(), 0, "the Julian calendar has no year 0");
    }

    /// Ethiopic → Gregorian → Ethiopic is the identity.
    #[test]
    fn cross_calendar_roundtrip(d in any_ethiopian_date()) {
        let g = d.to_gregorian();
        prop_assert_eq!(EthiopianDate::from_gregorian(g).unwrap(), d);
        let j = d.to_julian().unwrap();
        prop_assert_eq!(EthiopianDate::from_julian(j).unwrap(), d);
    }

    /// Any date produced from a fixed day number is *structurally* valid.
    #[test]
    fn produced_dates_are_always_well_formed(rd in RD_RANGE) {
        let d = EthiopianDate::from_rata_die(RataDie::new(rd)).unwrap();
        prop_assert!((1..=13).contains(&d.month()));
        prop_assert!(d.day() >= 1);
        prop_assert!(d.day() <= days_in_month(d.year(), d.month()));
        prop_assert!(d.day_of_year() >= 1);
        prop_assert!(d.day_of_year() <= days_in_year(d.year()));
        // Round-tripping through the constructor must be lossless.
        prop_assert_eq!(
            EthiopianDate::from_ymd(d.year(), d.month(), d.day()).unwrap(),
            d
        );
    }

    /// Ordering on dates agrees with ordering on fixed day numbers.
    #[test]
    fn ordering_is_chronological(a in any_ethiopian_date(), b in any_ethiopian_date()) {
        prop_assert_eq!(a.cmp(&b), a.to_rata_die().cmp(&b.to_rata_die()));
    }

    /// Adding then subtracting the same number of days is the identity.
    #[test]
    fn add_days_is_invertible(d in any_ethiopian_date(), n in -400_000i64..400_000) {
        let moved = d.add_days(n).unwrap();
        prop_assert_eq!(moved.sub_days(n).unwrap(), d);
        prop_assert_eq!(moved.days_since(d), n);
    }

    /// Successive days advance the weekday by exactly one.
    #[test]
    fn weekday_advances_monotonically(rd in RD_RANGE) {
        let a = RataDie::new(rd);
        prop_assert_eq!((a + 1).weekday(), a.weekday().succ());
        prop_assert_eq!((a - 1).weekday(), a.weekday().pred());
        prop_assert_eq!((a + 7).weekday(), a.weekday());
    }

    /// A year has 365 or 366 days, and exactly the leap years have 366.
    #[test]
    fn year_lengths(year in -3_000_000i32..3_000_000) {
        let start = EthiopianDate::from_ymd(year, 1, 1).unwrap().to_rata_die();
        let next = EthiopianDate::from_ymd(year + 1, 1, 1).unwrap().to_rata_die();
        let len = next - start;
        prop_assert_eq!(len, i64::from(days_in_year(year)));
        prop_assert_eq!(len == 366, year.rem_euclid(4) == 3);
        // Ṗagumēn is the only month whose length varies.
        prop_assert_eq!(days_in_month(year, 13), if len == 366 { 6 } else { 5 });
        for m in 1..=12u8 {
            prop_assert_eq!(days_in_month(year, m), 30);
        }
    }

    /// The Ethiopian new year is 11 or 12 September (Gregorian) for the whole
    /// span in which the two calendars are used side by side.
    #[test]
    fn enkutatash_is_11_or_12_september(year in 1900i32..2100) {
        let g = EthiopianDate::from_ymd(year, 1, 1).unwrap().to_gregorian();
        prop_assert_eq!(g.month(), 9);
        prop_assert!(g.day() == 11 || g.day() == 12);
        // 12 September exactly when the *following* Gregorian year is leap.
        prop_assert_eq!(g.day() == 12, ethiopic_calendar::gregorian::is_leap_year(g.year() + 1));
    }

    /// Day-of-year and (month, day) are two views of the same thing.
    #[test]
    fn day_of_year_is_consistent(d in any_ethiopian_date()) {
        let same = EthiopianDate::from_year_day(d.year(), d.day_of_year()).unwrap();
        prop_assert_eq!(same, d);
    }

    /// Month arithmetic never produces an invalid date and is monotone.
    #[test]
    fn add_months_is_well_behaved(d in any_ethiopian_date(), n in -100_000i64..100_000) {
        let moved = d.add_months(n).unwrap();
        prop_assert!(moved.day() <= days_in_month(moved.year(), moved.month()));
        let months = i64::from(moved.year()) * 13 + i64::from(moved.month() - 1)
            - (i64::from(d.year()) * 13 + i64::from(d.month() - 1));
        prop_assert_eq!(months, n);
    }

    /// The three fixed-day-number encodings are affine transforms of each other.
    #[test]
    fn jdn_mjd_unix_are_consistent(rd in RD_RANGE) {
        let r = RataDie::new(rd);
        prop_assert_eq!(r.to_jdn() - r.to_mjd(), 2_400_001);
        prop_assert_eq!(RataDie::from_jdn(r.to_jdn()), r);
        prop_assert_eq!(RataDie::from_mjd(r.to_mjd()), r);
        prop_assert_eq!(RataDie::from_unix_days(r.to_unix_days()), r);
    }

    /// Parsing the `Display` output returns the original date.
    #[test]
    fn display_parse_roundtrip(d in any_ethiopian_date()) {
        let s = d.to_string();
        prop_assert_eq!(s.parse::<EthiopianDate>().unwrap(), d);
    }

    /// The clock layer is lossless in both directions.
    #[test]
    fn clock_roundtrip(secs in 0u32..86_400) {
        let t = EthiopianTime::from_seconds_since_midnight(secs).unwrap();
        prop_assert_eq!(t.seconds_since_midnight(), secs);
        let back = EthiopianTime::from_ethiopian(
            t.ethiopian_hour(), t.minute(), t.second(), t.period()).unwrap();
        prop_assert_eq!(back, t);
        prop_assert!((1..=12).contains(&t.ethiopian_hour()));
        prop_assert_eq!(t.period().is_daytime(), (6..18).contains(&t.wall_hour()));
    }

    /// Timestamp → local Ethiopian date/time → timestamp is the identity for
    /// both day-boundary conventions.
    #[test]
    fn datetime_roundtrip(secs in -60_000_000_000i64..60_000_000_000, offset in -50_400i32..50_400) {
        for day_start in [DayStart::Dawn, DayStart::Midnight] {
            let dt = EthiopianDateTime::from_unix_timestamp(secs, offset, day_start).unwrap();
            prop_assert_eq!(dt.to_unix_timestamp(), secs);
        }
    }

    /// Before dawn, the traditional day boundary reports the previous date and
    /// the civil one does not; from dawn on they agree.
    #[test]
    fn day_start_differs_only_before_dawn(secs in -60_000_000_000i64..60_000_000_000) {
        let offset = ethiopic_calendar::clock::EAT_UTC_OFFSET_SECONDS;
        let dawn = EthiopianDateTime::from_unix_timestamp(secs, offset, DayStart::Dawn).unwrap();
        let civil = EthiopianDateTime::from_unix_timestamp(secs, offset, DayStart::Midnight).unwrap();
        let delta = civil.date().days_since(dawn.date());
        prop_assert_eq!(delta, i64::from(dawn.time().is_before_dawn()));
        prop_assert_eq!(
            dawn.time().is_before_dawn(),
            dawn.time().period() == Period::Lelit
        );
    }

    /// Every movable feast keeps its weekday, for any year.
    #[test]
    fn computus_weekday_invariants(year in -2_000_000i32..2_000_000) {
        let bh = BahireHasab::new(year).unwrap();
        prop_assert_eq!(bh.feast(MovableFeast::Nineveh).unwrap().weekday(), Weekday::Monday);
        prop_assert_eq!(bh.feast(MovableFeast::Tinsae).unwrap().weekday(), Weekday::Sunday);
        prop_assert_eq!(bh.feast(MovableFeast::Hosanna).unwrap().weekday(), Weekday::Sunday);
        prop_assert_eq!(bh.feast(MovableFeast::Siklet).unwrap().weekday(), Weekday::Friday);
        prop_assert_eq!(bh.feast(MovableFeast::Erget).unwrap().weekday(), Weekday::Thursday);
        prop_assert_eq!(bh.feast(MovableFeast::Paraclete).unwrap().weekday(), Weekday::Sunday);
    }

    /// Fasika always falls in the Ethiopian months Megabit, Miyazya or Ginbot,
    /// i.e. the Julian 22 March – 25 April window, and Lent is always 55 days
    /// before it.
    #[test]
    fn fasika_falls_in_the_paschal_window(year in 1i32..30_000) {
        let bh = BahireHasab::new(year).unwrap();
        let fasika = bh.feast(MovableFeast::Tinsae).unwrap();
        let j = fasika.to_julian().unwrap();
        let ok = (j.month() == 3 && j.day() >= 22) || (j.month() == 4 && j.day() <= 25);
        prop_assert!(ok, "Fasika of {} A.M. fell on {} (Julian)", year, j);
        prop_assert_eq!(
            fasika.days_since(bh.feast(MovableFeast::AbiyTsome).unwrap()),
            55
        );
    }

    /// The computus is periodic with the 532-year Alexandrian cycle: the whole
    /// paschal table repeats exactly, offset by a whole number of days.
    #[test]
    fn computus_has_the_532_year_period(year in 1i32..100_000) {
        let a = BahireHasab::new(year).unwrap();
        let b = BahireHasab::new(year + 532).unwrap();
        prop_assert_eq!(a.metqi(), b.metqi());
        prop_assert_eq!(a.abektie(), b.abektie());
        let da = a.feast(MovableFeast::Tinsae).unwrap();
        let db = b.feast(MovableFeast::Tinsae).unwrap();
        prop_assert_eq!((da.month(), da.day()), (db.month(), db.day()));
    }

    /// Formatting never panics and `%F` always parses back.
    #[test]
    fn formatting_roundtrip(d in any_ethiopian_date()) {
        use ethiopic_calendar::format::{format_date, Locale};
        let s = format_date(d, "%F", Locale::Amharic);
        prop_assert_eq!(s.parse::<EthiopianDate>().unwrap(), d);
        let long = format_date(d, "%A %-d %B %Y %N (%G) %j %J %R", Locale::Amharic);
        prop_assert!(!long.is_empty());
    }

    /// Civil-range sanity: every calendar agrees on the weekday.
    #[test]
    fn all_calendars_agree_on_the_weekday(rd in CIVIL_RD_RANGE) {
        let r = RataDie::new(rd);
        let e = EthiopianDate::from_rata_die(r).unwrap();
        let g = GregorianDate::from_rata_die(r).unwrap();
        let j = JulianDate::from_rata_die(r).unwrap();
        prop_assert_eq!(e.weekday(), g.weekday());
        prop_assert_eq!(e.weekday(), j.weekday());
    }
}
