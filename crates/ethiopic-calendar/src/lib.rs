//! # `ethiopic-calendar`
//!
//! Proleptic Ethiopian (Ge'ez / `Ethiopic`) calendar arithmetic with an obsessive
//! focus on being *correct at the boundaries*, where most implementations break.
//!
//! ## What this crate gets right
//!
//! * **The leap-year phase.** An Ethiopian year `y` is a leap year (i.e. the
//!   thirteenth month, *Ṗagumēn*, has six days) when `y mod 4 == 3`, **not**
//!   `y mod 4 == 0`. The Ethiopian year runs ahead of the Julian leap day, so
//!   the sixth epagomenal day falls in the Ethiopian year *preceding* the
//!   Gregorian/Julian leap year. Implementations that copy the Gregorian rule
//!   are off by one year for three quarters of the cycle.
//! * **The epoch / JDN offset.** Meskerem 1, 1 *Amete Mihret* is
//!   JDN 1 724 221 = 29 August 8 CE in the proleptic **Julian** calendar =
//!   27 August 8 CE in the proleptic **Gregorian** calendar = R.D. 2796.
//!   A very common bug is to anchor the epoch on the Gregorian date, which
//!   silently shifts every conversion before 1900 by two days.
//! * **Negative and zero years.** All division is Euclidean
//!   ([`i64::div_euclid`] / [`i64::rem_euclid`]), so the proleptic extension
//!   backwards past the epoch has no discontinuity at year 0. The Julian
//!   calendar, which genuinely has no year 0, rejects it explicitly
//!   ([`JulianDate`]).
//! * **Overflow.** Every intermediate is computed in [`i64`] while the public
//!   year type is [`i32`], so the entire representable year range is exact and
//!   panic-free.
//!
//! ## Layers
//!
//! | Layer | Module | Opt-in |
//! |-------|--------|--------|
//! | Day arithmetic | [`ethiopic`], [`gregorian`], [`julian`], [`rata_die`] | always |
//! | Era handling (Amete Mihret / Amete Alem) | [`era`] | always |
//! | Names and Ge'ez numerals | [`names`] | `alloc` for numerals |
//! | `strftime`-style formatting | [`format`] | `alloc` |
//! | Baḥire Ḥasab computus and holidays | [`bahire_hasab`], [`holidays`] | always |
//! | Ethiopian clock time (dawn-based, 12-hour) | [`clock`] | always, separate type |
//!
//! ## Quick start
//!
//! ```
//! use ethiopic_calendar::{EthiopianDate, GregorianDate};
//!
//! let g = GregorianDate::from_ymd(2024, 9, 11).unwrap();
//! let e = EthiopianDate::from_gregorian(g).unwrap();
//! assert_eq!((e.year(), e.month(), e.day()), (2017, 1, 1)); // Enkutatash
//! assert!(!e.is_leap_year());
//!
//! // The year before is the leap year: Ṗagumēn had six days.
//! let p = EthiopianDate::from_ymd(2015, 13, 6).unwrap();
//! assert_eq!(p.to_gregorian().as_tuple(), (2023, 9, 11));
//! ```
//!
//! ## Algorithmic references
//!
//! * E. M. Reingold & N. Dershowitz, *Calendrical Calculations: The Ultimate
//!   Edition*, 4th ed., Cambridge University Press, 2018 — chapter 4
//!   ("The Coptic and Ethiopic Calendars") and chapter 2 (Julian).
//! * O. Neugebauer, *Ethiopic Astronomy and Computus*, Österreichische Akademie
//!   der Wissenschaften, 1979 — the *Baḥire Ḥasab* parameters (*wenber*,
//!   *abektē*, *meṭq*, *tewsak*).
//! * J.-B. Chabot / A. Grohmann on the Alexandrian 532-year paschal cycle
//!   underlying both the Coptic and Ethiopic computus.
//! * J. Meeus, *Astronomical Algorithms*, 2nd ed., 1998, ch. 7–8 — Julian Day
//!   Number conventions (JDN is defined at **noon** UT; this crate's
//!   `to_jdn` returns that integer).
//!
//! See `docs/ALGORITHMS.md` in the repository for the derivations, including
//! the empirical validation of the *tewsak* table against the Julian computus.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bahire_hasab;
pub mod clock;
pub mod era;
pub mod error;
pub mod ethiopic;
pub mod gregorian;
pub mod holidays;
pub mod julian;
pub mod names;
pub mod rata_die;
pub mod weekday;

#[cfg(feature = "alloc")]
pub mod format;

mod parse;

pub use bahire_hasab::{BahireHasab, MovableFeast};
pub use clock::{DayStart, EthiopianDateTime, EthiopianTime, Period};
pub use era::Era;
pub use error::{Error, Result};
pub use ethiopic::{EthiopianDate, Month};
pub use gregorian::GregorianDate;
pub use holidays::Holiday;
pub use julian::JulianDate;
pub use rata_die::RataDie;
pub use weekday::Weekday;

/// Version of this crate, as reported by Cargo at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
