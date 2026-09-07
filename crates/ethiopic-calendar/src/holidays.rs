//! Ethiopian holidays: the fixed-date ones, the movable ones from the computus,
//! and — importantly — the difference between a holiday's *calendar* date and
//! its *observed* date.
//!
//! # Scope, honestly stated
//!
//! Ethiopia's public holiday list also contains three Islamic observances (Eid
//! al-Fitr, Eid al-Adha and Mawlid). Those follow the Hijri lunar calendar and,
//! in Ethiopian practice, depend on local moon sighting; they are **not**
//! computable from the Ethiopian calendar and this crate deliberately does not
//! guess at them. See [`UNSUPPORTED_ISLAMIC_HOLIDAYS`].
//!
//! # Observance versus calendar date
//!
//! Genna (Christmas) is the interesting case. Its calendar date is 29 Taḫśaś,
//! but in years where that would land on 8 January — i.e. when the following
//! Gregorian year is a leap year — most of the country observes it a day
//! earlier, on 28 Taḫśaś, so that Genna stays on 7 January. Lalibela, and
//! churches dedicated to *Bä'alä Ǝgziabḥer*, keep the canonical 29 Taḫśaś.
//! [`Holiday::date`] gives the calendar date; [`Holiday::observed_date`] applies
//! the shift.
//!
//! Timkat, by contrast, is *not* adjusted: it stays on 11 Ṭərr and therefore
//! moves between 19 and 20 January in the Gregorian calendar.

use crate::bahire_hasab::{BahireHasab, MovableFeast};
use crate::error::Result;
use crate::ethiopic::EthiopianDate;

/// Islamic holidays observed in Ethiopia that this crate does not compute.
pub const UNSUPPORTED_ISLAMIC_HOLIDAYS: [&str; 3] = ["Eid al-Fitr", "Eid al-Adha", "Mawlid"];

/// An Ethiopian holiday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Holiday {
    /// እንቁጣጣሽ — New Year, 1 Meskerem.
    Enkutatash,
    /// መስቀል — Finding of the True Cross, 17 Meskerem.
    Meskel,
    /// ገና — Christmas, 29 Taḫśaś (observed 28 Taḫśaś in some years).
    Genna,
    /// ጥምቀት — Epiphany, 11 Ṭərr.
    Timkat,
    /// የአድዋ ድል በዓል — Victory of Adwa, 23 Yäkatit.
    AdwaVictoryDay,
    /// የሠራተኞች ቀን — International Labour Day, 23 Miyazya.
    LabourDay,
    /// የአርበኞች ቀን — Patriots' Victory Day, 27 Miyazya.
    PatriotsDay,
    /// ደርግ የወደቀበት ቀን — Downfall of the Derg, 20 Gənbot.
    DergDownfallDay,
    /// ስቅለት — Good Friday (movable).
    GoodFriday,
    /// ፋሲካ — Easter (movable).
    Fasika,
}

/// Every holiday this crate knows how to compute.
pub const ALL_HOLIDAYS: [Holiday; 10] = [
    Holiday::Enkutatash,
    Holiday::Meskel,
    Holiday::Genna,
    Holiday::Timkat,
    Holiday::AdwaVictoryDay,
    Holiday::LabourDay,
    Holiday::PatriotsDay,
    Holiday::DergDownfallDay,
    Holiday::GoodFriday,
    Holiday::Fasika,
];

/// How a holiday's date is determined.
///
/// Modelled as one exhaustive enum rather than two `Option`-returning
/// accessors so that "fixed or movable" is a fact the type system knows, and
/// [`Holiday::date`] needs no unreachable branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// A fixed `(month, day)` in every year.
    Fixed(u8, u8),
    /// Derived from the computus.
    Movable(MovableFeast),
}

impl Holiday {
    /// How this holiday's date is determined.
    #[must_use]
    pub const fn rule(self) -> Rule {
        match self {
            Self::Enkutatash => Rule::Fixed(1, 1),
            Self::Meskel => Rule::Fixed(1, 17),
            Self::Genna => Rule::Fixed(4, 29),
            Self::Timkat => Rule::Fixed(5, 11),
            Self::AdwaVictoryDay => Rule::Fixed(6, 23),
            Self::LabourDay => Rule::Fixed(8, 23),
            Self::PatriotsDay => Rule::Fixed(8, 27),
            Self::DergDownfallDay => Rule::Fixed(9, 20),
            Self::GoodFriday => Rule::Movable(MovableFeast::Siklet),
            Self::Fasika => Rule::Movable(MovableFeast::Tinsae),
        }
    }

    /// The fixed `(month, day)` of this holiday, or `None` if it is movable.
    #[must_use]
    pub const fn fixed_date(self) -> Option<(u8, u8)> {
        match self.rule() {
            Rule::Fixed(m, d) => Some((m, d)),
            Rule::Movable(_) => None,
        }
    }

    /// The movable feast backing this holiday, if any.
    #[must_use]
    pub const fn movable_feast(self) -> Option<MovableFeast> {
        match self.rule() {
            Rule::Movable(f) => Some(f),
            Rule::Fixed(..) => None,
        }
    }

    /// The holiday's date in the Ethiopian calendar for a given year.
    ///
    /// This is the *calendar* date. For Genna, see [`Self::observed_date`].
    pub fn date(self, year: i32) -> Result<EthiopianDate> {
        match self.rule() {
            Rule::Fixed(m, d) => EthiopianDate::from_ymd(year, m, d),
            Rule::Movable(feast) => BahireHasab::new(year)?.feast(feast),
        }
    }

    /// The date the holiday is actually observed nationally.
    ///
    /// Identical to [`Self::date`] except for Genna in the years where
    /// 29 Taḫśaś would fall on 8 January, in which case it returns 28 Taḫśaś.
    ///
    /// ```
    /// use ethiopic_calendar::holidays::Holiday;
    ///
    /// // 2016 E.C.: the calendar date is 8 January 2024, observed 7 January.
    /// assert_eq!(Holiday::Genna.date(2016).unwrap().to_gregorian().as_tuple(), (2024, 1, 8));
    /// assert_eq!(
    ///     Holiday::Genna.observed_date(2016).unwrap().to_gregorian().as_tuple(),
    ///     (2024, 1, 7)
    /// );
    /// ```
    pub fn observed_date(self, year: i32) -> Result<EthiopianDate> {
        let date = self.date(year)?;
        let g = date.to_gregorian();
        if self == Self::Genna && (g.month(), g.day()) == (1, 8) {
            return date.sub_days(1);
        }
        Ok(date)
    }

    /// Whether the day is a national public holiday (all of the ones modelled
    /// here are).
    #[must_use]
    pub const fn is_public_holiday(self) -> bool {
        true
    }

    /// Amharic name.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::Enkutatash => "እንቁጣጣሽ",
            Self::Meskel => "መስቀል",
            Self::Genna => "ገና",
            Self::Timkat => "ጥምቀት",
            Self::AdwaVictoryDay => "የአድዋ ድል በዓል",
            Self::LabourDay => "የሠራተኞች ቀን",
            Self::PatriotsDay => "የአርበኞች ቀን",
            Self::DergDownfallDay => "ደርግ የወደቀበት ቀን",
            Self::GoodFriday => "ስቅለት",
            Self::Fasika => "ፋሲካ",
        }
    }

    /// English name.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::Enkutatash => "Ethiopian New Year",
            Self::Meskel => "Finding of the True Cross",
            Self::Genna => "Ethiopian Christmas",
            Self::Timkat => "Epiphany",
            Self::AdwaVictoryDay => "Victory of Adwa",
            Self::LabourDay => "International Labour Day",
            Self::PatriotsDay => "Patriots' Victory Day",
            Self::DergDownfallDay => "Downfall of the Derg",
            Self::GoodFriday => "Good Friday",
            Self::Fasika => "Easter",
        }
    }

    /// Stable machine-readable key, e.g. `"enkutatash"`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Enkutatash => "enkutatash",
            Self::Meskel => "meskel",
            Self::Genna => "genna",
            Self::Timkat => "timkat",
            Self::AdwaVictoryDay => "adwa",
            Self::LabourDay => "labour_day",
            Self::PatriotsDay => "patriots_day",
            Self::DergDownfallDay => "derg_downfall",
            Self::GoodFriday => "good_friday",
            Self::Fasika => "fasika",
        }
    }
}

/// Every holiday falling on `date`, using observed dates.
pub fn holidays_on(date: EthiopianDate) -> impl Iterator<Item = Holiday> {
    ALL_HOLIDAYS
        .into_iter()
        .filter(move |h| h.observed_date(date.year()) == Ok(date))
}

/// Whether `date` is a public holiday (observed).
#[must_use]
pub fn is_holiday(date: EthiopianDate) -> bool {
    holidays_on(date).next().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_holidays_against_published_gregorian_dates() {
        // (Ethiopian year, holiday, Gregorian date)
        let cases = [
            (2017, Holiday::Enkutatash, (2024, 9, 11)),
            (2017, Holiday::Meskel, (2024, 9, 27)),
            (2017, Holiday::Timkat, (2025, 1, 19)),
            (2017, Holiday::AdwaVictoryDay, (2025, 3, 2)),
            (2017, Holiday::LabourDay, (2025, 5, 1)),
            (2017, Holiday::PatriotsDay, (2025, 5, 5)),
            (2017, Holiday::DergDownfallDay, (2025, 5, 28)),
            // A year where the Gregorian side shifts by one day.
            (2016, Holiday::Enkutatash, (2023, 9, 12)),
            (2016, Holiday::Timkat, (2024, 1, 20)),
            (2016, Holiday::LabourDay, (2024, 5, 1)),
        ];
        for (year, holiday, greg) in cases {
            assert_eq!(
                holiday.date(year).unwrap().to_gregorian().as_tuple(),
                greg,
                "{} of {year} E.C.",
                holiday.english()
            );
        }
    }

    #[test]
    fn genna_is_always_observed_on_7_january() {
        for year in 1900..2100 {
            let observed = Holiday::Genna.observed_date(year).unwrap().to_gregorian();
            assert_eq!(
                (observed.month(), observed.day()),
                (1, 7),
                "Genna of {year} E.C. observed on {observed}"
            );
        }
    }

    #[test]
    fn genna_calendar_date_is_29_tahsas_except_when_shifted() {
        for year in 1900..2100 {
            let canonical = Holiday::Genna.date(year).unwrap();
            assert_eq!((canonical.month(), canonical.day()), (4, 29));
            let observed = Holiday::Genna.observed_date(year).unwrap();
            let shifted = observed != canonical;
            // The shift happens exactly when the Ethiopian year is ≡ 0 (mod 4),
            // i.e. the year of John, the one after the leap year.
            assert_eq!(shifted, year.rem_euclid(4) == 0, "year {year}");
        }
    }

    #[test]
    fn timkat_is_19_or_20_january() {
        for year in 1900..2100 {
            let g = Holiday::Timkat.date(year).unwrap().to_gregorian();
            assert_eq!(g.month(), 1);
            assert!(
                g.day() == 19 || g.day() == 20,
                "Timkat of {year} E.C. fell on {g}"
            );
            assert_eq!(g.day() == 20, year.rem_euclid(4) == 0);
        }
    }

    #[test]
    fn movable_holidays_track_the_computus() {
        let bh = BahireHasab::new(2017).unwrap();
        assert_eq!(
            Holiday::Fasika.date(2017).unwrap(),
            bh.feast(MovableFeast::Tinsae).unwrap()
        );
        assert_eq!(
            Holiday::GoodFriday
                .date(2017)
                .unwrap()
                .to_gregorian()
                .as_tuple(),
            (2025, 4, 18)
        );
    }

    #[test]
    fn lookup_by_date() {
        let new_year = EthiopianDate::from_ymd(2017, 1, 1).unwrap();
        assert!(is_holiday(new_year));
        let mut it = holidays_on(new_year);
        assert_eq!(it.next(), Some(Holiday::Enkutatash));
        assert_eq!(it.next(), None);
        let ordinary = EthiopianDate::from_ymd(2017, 2, 3).unwrap();
        assert!(!is_holiday(ordinary));
    }

    #[test]
    fn every_key_is_unique() {
        for (i, a) in ALL_HOLIDAYS.iter().enumerate() {
            for b in &ALL_HOLIDAYS[i + 1..] {
                assert_ne!(a.key(), b.key());
                assert_ne!(a.english(), b.english());
            }
        }
    }
}
