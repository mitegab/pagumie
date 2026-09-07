//! **Baḥire Ḥasab** (ባሕረ ሐሳብ, "sea of computation") — the Ethiopian computus.
//!
//! This is the Alexandrian 532-year paschal cycle as transmitted through the
//! Ethiopian Orthodox Täwaḥədo Church: a 19-year lunar (Metonic) cycle crossed
//! with the 28-year Julian solar cycle. From an *Amete Alem* year it derives
//! the epact-like parameters *abektē* and *meṭq*, from those the date of the
//! Fast of Nineveh, and from Nineveh every movable feast of the year by a fixed
//! table of offsets (*tewsak*).
//!
//! # The computation
//!
//! ```text
//! AA      = year + 5500                     Amete Alem
//! medeb   = AA mod 19                       position in the Metonic cycle
//! wenber  = medeb - 1  (18 if medeb = 0)    "the seat"
//! abektē  = (wenber · 11) mod 30            solar epact
//! meṭq    = (wenber · 19) mod 30            lunar epact   (abektē + meṭq = 30)
//! ```
//!
//! *Bä'alä Meṭq* — the day the meṭq falls on — is Meskerem *meṭq* when
//! `meṭq > 14`, and Ṭiqimt *meṭq* otherwise. (When `meṭq = 0`, which happens
//! once per Metonic cycle, it is Meskerem 30; this degenerate case is where
//! nearly every published implementation diverges, and it is verified below.)
//!
//! Adding the *tewsak* of the weekday of Bä'alä Meṭq, plus 120 days, lands on
//! *Nineveh* (ጾመ ነነዌ), which is always a Monday:
//!
//! | weekday of Bä'alä Meṭq | Sun | Mon | Tue | Wed | Thu | Fri | Sat |
//! |---|---|---|---|---|---|---|---|
//! | tewsak | 7 | 6 | 5 | 4 | 3 | 2 | 8 |
//!
//! # Validation
//!
//! Ethiopian *Fasika* is the Alexandrian Easter, which is also Eastern Orthodox
//! Easter. The test suite therefore checks this implementation against an
//! *independent* algorithm — the Meeus Julian-calendar paschal computus — for
//! **every year from 1 to 3000 A.M.**, and against published Fasika dates.
//! Two derivations, different in form, agreeing on 3000 years, is the strongest
//! evidence available short of a manuscript.
//!
//! # References
//!
//! * O. Neugebauer, *Ethiopic Astronomy and Computus*, Vienna, 1979.
//! * A. Grohmann, "Die im Äthiopischen, Arabischen und Koptischen erhaltenen
//!   Visionen Apa Schenute", on the Alexandrian cycle.
//! * G. Fiaccadori, "Bäḥrä ḥassab", in *Encyclopaedia Aethiopica* vol. 1.
//! * J. Meeus, *Astronomical Algorithms*, 2nd ed., §8 (Julian Easter), used as
//!   the independent cross-check.

use crate::error::Result;
use crate::ethiopic::EthiopianDate;
use crate::weekday::Weekday;

/// The four-year cycle of evangelists; the leap year is always Luke's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Evangelist {
    /// ዘመነ ማቴዎስ — Year of Matthew.
    Matewos,
    /// ዘመነ ማርቆስ — Year of Mark.
    Marqos,
    /// ዘመነ ሉቃስ — Year of Luke; always the leap year (Ṗagumēn has six days).
    Luqas,
    /// ዘመነ ዮሐንስ — Year of John.
    Yohannes,
}

impl Evangelist {
    /// Amharic name of the year, e.g. `"ዘመነ ሉቃስ"`.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::Matewos => "ዘመነ ማቴዎስ",
            Self::Marqos => "ዘመነ ማርቆስ",
            Self::Luqas => "ዘመነ ሉቃስ",
            Self::Yohannes => "ዘመነ ዮሐንስ",
        }
    }

    /// English name, e.g. `"Year of Luke"`.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::Matewos => "Year of Matthew",
            Self::Marqos => "Year of Mark",
            Self::Luqas => "Year of Luke",
            Self::Yohannes => "Year of John",
        }
    }
}

/// A movable feast or fast, defined by its offset in days from Nineveh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovableFeast {
    /// ጾመ ነነዌ — Fast of Nineveh (always a Monday). Offset 0.
    Nineveh,
    /// ዓቢይ ጾም — the Great Lent begins. Offset 14.
    AbiyTsome,
    /// ደብረ ዘይት — Mount of Olives, mid-Lent. Offset 41.
    DebreZeit,
    /// ሆሳዕና — Palm Sunday. Offset 62.
    Hosanna,
    /// ስቅለት — Good Friday. Offset 67.
    Siklet,
    /// ትንሣኤ / ፋሲካ — Easter. Offset 69.
    Tinsae,
    /// ርክበ ካህናት — Assembly of the Priests. Offset 93.
    RikbeKahnat,
    /// ዕርገት — Ascension. Offset 108.
    Erget,
    /// ጰራቅሊጦስ — Pentecost. Offset 118.
    Paraclete,
    /// ጾመ ሐዋርያት — Fast of the Apostles. Offset 119.
    TsomeHawaryat,
    /// ጾመ ድህነት — Fast of Salvation (first Wednesday after Pentecost). Offset 121.
    TsomeDihnet,
}

/// Every movable feast, in chronological order.
pub const ALL_MOVABLE_FEASTS: [MovableFeast; 11] = [
    MovableFeast::Nineveh,
    MovableFeast::AbiyTsome,
    MovableFeast::DebreZeit,
    MovableFeast::Hosanna,
    MovableFeast::Siklet,
    MovableFeast::Tinsae,
    MovableFeast::RikbeKahnat,
    MovableFeast::Erget,
    MovableFeast::Paraclete,
    MovableFeast::TsomeHawaryat,
    MovableFeast::TsomeDihnet,
];

impl MovableFeast {
    /// Days after the Fast of Nineveh.
    #[must_use]
    pub const fn offset_from_nineveh(self) -> i64 {
        match self {
            Self::Nineveh => 0,
            Self::AbiyTsome => 14,
            Self::DebreZeit => 41,
            Self::Hosanna => 62,
            Self::Siklet => 67,
            Self::Tinsae => 69,
            Self::RikbeKahnat => 93,
            Self::Erget => 108,
            Self::Paraclete => 118,
            Self::TsomeHawaryat => 119,
            Self::TsomeDihnet => 121,
        }
    }

    /// Amharic name.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::Nineveh => "ጾመ ነነዌ",
            Self::AbiyTsome => "ዓቢይ ጾም",
            Self::DebreZeit => "ደብረ ዘይት",
            Self::Hosanna => "ሆሳዕና",
            Self::Siklet => "ስቅለት",
            Self::Tinsae => "ትንሣኤ",
            Self::RikbeKahnat => "ርክበ ካህናት",
            Self::Erget => "ዕርገት",
            Self::Paraclete => "ጰራቅሊጦስ",
            Self::TsomeHawaryat => "ጾመ ሐዋርያት",
            Self::TsomeDihnet => "ጾመ ድህነት",
        }
    }

    /// English name.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::Nineveh => "Fast of Nineveh",
            Self::AbiyTsome => "Great Lent",
            Self::DebreZeit => "Debre Zeit",
            Self::Hosanna => "Palm Sunday",
            Self::Siklet => "Good Friday",
            Self::Tinsae => "Easter",
            Self::RikbeKahnat => "Rikbe Kahnat",
            Self::Erget => "Ascension",
            Self::Paraclete => "Pentecost",
            Self::TsomeHawaryat => "Fast of the Apostles",
            Self::TsomeDihnet => "Fast of Salvation",
        }
    }

    /// Stable machine-readable key, e.g. `"tinsae"` — used by the CLI and the
    /// D-Bus/JSON surfaces so downstream parsers never depend on a localised
    /// name.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Nineveh => "nineveh",
            Self::AbiyTsome => "abiy_tsome",
            Self::DebreZeit => "debre_zeit",
            Self::Hosanna => "hosanna",
            Self::Siklet => "siklet",
            Self::Tinsae => "tinsae",
            Self::RikbeKahnat => "rikbe_kahnat",
            Self::Erget => "erget",
            Self::Paraclete => "paraclete",
            Self::TsomeHawaryat => "tsome_hawaryat",
            Self::TsomeDihnet => "tsome_dihnet",
        }
    }
}

/// The *tewsak* added to Bä'alä Meṭq, indexed by weekday (Sunday = 0).
///
/// Saturday's value is 8 rather than 1: the count is inclusive of the starting
/// day and never yields the same week, which is why a naive "next Monday"
/// implementation is right six times in seven and wrong the seventh.
pub const TEWSAK: [i64; 7] = [7, 6, 5, 4, 3, 2, 8];

/// Days from Bä'alä Meṭq to Nineveh, before the tewsak is added.
const MEBAJA_HAMER_BASE: i64 = 120;

/// The computus for one Ethiopian year.
///
/// ```
/// use ethiopic_calendar::bahire_hasab::{BahireHasab, MovableFeast};
///
/// let bh = BahireHasab::new(2017).unwrap();
/// assert_eq!(bh.amete_alem(), 7517);
/// // Fasika 2017 E.C. = 20 April 2025.
/// let fasika = bh.feast(MovableFeast::Tinsae).unwrap();
/// assert_eq!(fasika.to_gregorian().as_tuple(), (2025, 4, 20));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BahireHasab {
    year: i32,
    amete_alem: i64,
    medeb: u8,
    wenber: u8,
    abektie: u8,
    metqi: u8,
    beale_metqi: EthiopianDate,
    tewsak: i64,
    nineveh: EthiopianDate,
}

impl BahireHasab {
    /// Runs the computus for an *Amete Mihret* year.
    pub fn new(year: i32) -> Result<Self> {
        let amete_alem = i64::from(year) + crate::era::AMETE_ALEM_OFFSET;
        let medeb = amete_alem.rem_euclid(19);
        let wenber = if medeb == 0 { 18 } else { medeb - 1 };
        let abektie = (wenber * 11).rem_euclid(30);
        let metqi = (wenber * 19).rem_euclid(30);

        // Bä'alä Meṭq: Meskerem if the meṭq is past mid-month, else Ṭiqimt.
        // meṭq = 0 is the degenerate case and belongs to Meskerem 30.
        let (bm_month, bm_day) = if metqi == 0 {
            (1u8, 30u8)
        } else if metqi > 14 {
            (1u8, metqi as u8)
        } else {
            (2u8, metqi as u8)
        };
        let beale_metqi = EthiopianDate::from_ymd(year, bm_month, bm_day)?;
        let tewsak = TEWSAK[beale_metqi.weekday().number() as usize];
        let nineveh = beale_metqi.add_days(MEBAJA_HAMER_BASE + tewsak)?;

        Ok(Self {
            year,
            amete_alem,
            medeb: medeb as u8,
            wenber: wenber as u8,
            abektie: abektie as u8,
            metqi: metqi as u8,
            beale_metqi,
            tewsak,
            nineveh,
        })
    }

    /// The *Amete Mihret* year this computus is for.
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// The *Amete Alem* year, `year + 5500`.
    #[must_use]
    pub const fn amete_alem(&self) -> i64 {
        self.amete_alem
    }

    /// *Medeb* — position in the 19-year Metonic cycle, `AA mod 19`.
    #[must_use]
    pub const fn medeb(&self) -> u8 {
        self.medeb
    }

    /// *Wenber* — `medeb - 1`, or 18 when `medeb` is 0.
    #[must_use]
    pub const fn wenber(&self) -> u8 {
        self.wenber
    }

    /// *Abektē* — the solar epact, `(wenber · 11) mod 30`.
    #[must_use]
    pub const fn abektie(&self) -> u8 {
        self.abektie
    }

    /// *Meṭq* — the lunar epact, `(wenber · 19) mod 30`.
    #[must_use]
    pub const fn metqi(&self) -> u8 {
        self.metqi
    }

    /// *Bä'alä Meṭq* — the date the meṭq falls on.
    #[must_use]
    pub const fn beale_metqi(&self) -> EthiopianDate {
        self.beale_metqi
    }

    /// The *tewsak* applied to Bä'alä Meṭq for this year.
    #[must_use]
    pub const fn tewsak(&self) -> i64 {
        self.tewsak
    }

    /// *Mebaja Hamer* — the traditional sum `meṭq + tewsak`.
    #[must_use]
    pub const fn mebaja_hamer(&self) -> i64 {
        self.metqi as i64 + self.tewsak
    }

    /// *Metene Rabiet* — `⌊AA / 4⌋`, the count of completed leap cycles.
    #[must_use]
    pub const fn metene_rabiet(&self) -> i64 {
        self.amete_alem.div_euclid(4)
    }

    /// The evangelist whose year this is. Luke's year is always the leap year.
    #[must_use]
    pub const fn evangelist(&self) -> Evangelist {
        match self.amete_alem.rem_euclid(4) {
            0 => Evangelist::Yohannes,
            1 => Evangelist::Matewos,
            2 => Evangelist::Marqos,
            _ => Evangelist::Luqas,
        }
    }

    /// Weekday of 1 Meskerem (Enkutatash) — traditionally *Tinte Yä'ämät*.
    #[must_use]
    pub fn new_year_weekday(&self) -> Weekday {
        self.beale_metqi.new_year().weekday()
    }

    /// The date of a movable feast in this year.
    pub fn feast(&self, feast: MovableFeast) -> Result<EthiopianDate> {
        self.nineveh.add_days(feast.offset_from_nineveh())
    }

    /// Every movable feast of the year, in chronological order.
    pub fn feasts(&self) -> Result<[(MovableFeast, EthiopianDate); 11]> {
        let mut out = [(MovableFeast::Nineveh, self.nineveh); 11];
        let mut i = 0;
        while i < ALL_MOVABLE_FEASTS.len() {
            let f = ALL_MOVABLE_FEASTS[i];
            out[i] = (f, self.feast(f)?);
            i += 1;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gregorian::GregorianDate;
    use crate::julian::JulianDate;
    use crate::rata_die::RataDie;

    /// Independent implementation: Meeus's Julian-calendar paschal computus.
    ///
    /// Deliberately written in a different style (epact arithmetic on the
    /// Gregorian year, then a k-day search) so that a shared misconception is
    /// unlikely to cancel out.
    fn orthodox_easter_rd(gregorian_year: i32) -> RataDie {
        let shifted_epact = (14 + 11 * i64::from(gregorian_year).rem_euclid(19)).rem_euclid(30);
        let j_year = if gregorian_year > 0 {
            gregorian_year
        } else {
            gregorian_year - 1
        };
        let paschal_moon =
            JulianDate::from_ymd(j_year, 4, 19).unwrap().to_rata_die() - shifted_epact;
        paschal_moon.kday_after(Weekday::Sunday)
    }

    #[test]
    fn parameters_of_2017_ec() {
        let bh = BahireHasab::new(2017).unwrap();
        assert_eq!(bh.amete_alem(), 7517);
        assert_eq!(i64::from(bh.medeb()), 7517 % 19);
        assert_eq!(i64::from(bh.wenber()), 7517 % 19 - 1);
        // abektē + meṭq == 30 whenever neither is 0.
        if bh.abektie() != 0 {
            assert_eq!(u16::from(bh.abektie()) + u16::from(bh.metqi()), 30);
        }
        assert_eq!(bh.evangelist(), Evangelist::Matewos);
    }

    #[test]
    fn epacts_are_complementary() {
        for year in 1..2000 {
            let bh = BahireHasab::new(year).unwrap();
            let sum = u16::from(bh.abektie()) + u16::from(bh.metqi());
            assert!(sum == 30 || sum == 0, "year {year}: abektie + metqi = {sum}");
        }
    }

    #[test]
    fn nineveh_is_always_a_monday() {
        for year in 1..3000 {
            let bh = BahireHasab::new(year).unwrap();
            assert_eq!(
                bh.feast(MovableFeast::Nineveh).unwrap().weekday(),
                Weekday::Monday,
                "year {year}"
            );
        }
    }

    #[test]
    fn easter_is_always_a_sunday_and_lent_starts_on_a_monday() {
        for year in 1..3000 {
            let bh = BahireHasab::new(year).unwrap();
            assert_eq!(
                bh.feast(MovableFeast::Tinsae).unwrap().weekday(),
                Weekday::Sunday,
                "year {year}"
            );
            assert_eq!(
                bh.feast(MovableFeast::Hosanna).unwrap().weekday(),
                Weekday::Sunday
            );
            assert_eq!(
                bh.feast(MovableFeast::Siklet).unwrap().weekday(),
                Weekday::Friday
            );
            assert_eq!(
                bh.feast(MovableFeast::AbiyTsome).unwrap().weekday(),
                Weekday::Monday
            );
        }
    }

    /// The headline cross-check: 3000 years against an independent algorithm.
    #[test]
    fn fasika_matches_the_julian_computus_for_three_millennia() {
        for year in 1..=3000i32 {
            let bh = BahireHasab::new(year).unwrap();
            let got = bh.feast(MovableFeast::Tinsae).unwrap().to_rata_die();
            let expected = orthodox_easter_rd(year + 8);
            assert_eq!(
                got,
                expected,
                "Fasika {year} A.M.: got {:?}, expected {:?}",
                GregorianDate::from_rata_die(got).unwrap(),
                GregorianDate::from_rata_die(expected).unwrap()
            );
        }
    }

    #[test]
    fn published_fasika_dates() {
        // Gregorian dates of Ethiopian Easter, from published liturgical
        // calendars (these are also the Eastern Orthodox Pascha dates).
        let cases = [
            (2015, (2023, 4, 16)),
            (2016, (2024, 5, 5)),
            (2017, (2025, 4, 20)),
            (2018, (2026, 4, 12)),
            (2019, (2027, 5, 2)),
        ];
        for (ec, greg) in cases {
            let bh = BahireHasab::new(ec).unwrap();
            assert_eq!(
                bh.feast(MovableFeast::Tinsae).unwrap().to_gregorian().as_tuple(),
                greg,
                "Fasika of {ec} E.C."
            );
        }
    }

    #[test]
    fn the_degenerate_metqi_zero_case_is_handled() {
        // meṭq = 0 occurs once per 19-year cycle; check a handful explicitly
        // against the independent computus.
        let mut seen = 0;
        for year in 1500..2100 {
            let bh = BahireHasab::new(year).unwrap();
            if bh.metqi() == 0 {
                seen += 1;
                assert_eq!(bh.beale_metqi().as_tuple(), (year, 1, 30));
                assert_eq!(
                    bh.feast(MovableFeast::Tinsae).unwrap().to_rata_die(),
                    orthodox_easter_rd(year + 8)
                );
            }
        }
        assert!(seen >= 30, "expected ~31 meṭq=0 years, saw {seen}");
    }

    #[test]
    fn luke_is_the_leap_year() {
        for year in -100..3000 {
            let bh = BahireHasab::new(year).unwrap();
            assert_eq!(
                bh.evangelist() == Evangelist::Luqas,
                crate::ethiopic::is_leap_year(year),
                "year {year}"
            );
        }
    }

    #[test]
    fn feasts_are_ordered_and_consistent() {
        let bh = BahireHasab::new(2018).unwrap();
        let feasts = bh.feasts().unwrap();
        for w in feasts.windows(2) {
            assert!(w[0].1 <= w[1].1);
        }
        assert_eq!(feasts[0].1, bh.feast(MovableFeast::Nineveh).unwrap());
        // Fasika is 55 days after the start of the Great Lent.
        let lent = bh.feast(MovableFeast::AbiyTsome).unwrap();
        let easter = bh.feast(MovableFeast::Tinsae).unwrap();
        assert_eq!(easter.days_since(lent), 55);
    }
}
