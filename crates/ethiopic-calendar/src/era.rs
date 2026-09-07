//! Ethiopian eras: *Amete Alem* and *Amete Mihret*.
//!
//! Ethiopian dates are quoted in one of two year counts:
//!
//! * **Amete Mihret** (ዓመተ ምሕረት, "Year of Mercy") — the incarnation era. This
//!   is what "2017 E.C." means in everyday use and is the crate's canonical
//!   representation.
//! * **Amete Alem** (ዓመተ ዓለም, "Year of the World") — the era of creation, used
//!   in liturgical documents and, crucially, as the *input to the Baḥire Ḥasab*
//!   computus. `AA = AM + 5500`.
//!
//! The offset is 5500 years exactly, following the Alexandrian world era
//! adopted by Annianus (creation placed 5500 years before the incarnation).
//! Note that this differs from the Byzantine world era (5508/5509) — mixing the
//! two is a common source of five-hundred-year errors in the computus.

use crate::error::{Error, Result};

/// Years between the *Amete Alem* and *Amete Mihret* epochs.
pub const AMETE_ALEM_OFFSET: i64 = 5500;

/// Which year count a date's year is expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum Era {
    /// ዓመተ ምሕረት — the incarnation era, the default.
    #[default]
    AmeteMihret,
    /// ዓመተ ዓለም — the world era, `AM + 5500`.
    AmeteAlem,
}

impl Era {
    /// Converts a year expressed in this era to *Amete Mihret*.
    pub const fn to_amete_mihret(self, year: i32) -> Result<i32> {
        let v = match self {
            Self::AmeteMihret => year as i64,
            Self::AmeteAlem => year as i64 - AMETE_ALEM_OFFSET,
        };
        if v < i32::MIN as i64 || v > i32::MAX as i64 {
            return Err(Error::YearOutOfRange { year: v });
        }
        Ok(v as i32)
    }

    /// Converts an *Amete Mihret* year into this era.
    ///
    /// Returns [`i64`] because Amete Alem years can exceed [`i32`] at the top of
    /// the supported range.
    #[must_use]
    pub const fn from_amete_mihret(self, year: i32) -> i64 {
        match self {
            Self::AmeteMihret => year as i64,
            Self::AmeteAlem => year as i64 + AMETE_ALEM_OFFSET,
        }
    }

    /// Amharic abbreviation: `"ዓ.ም."` or `"ዓ.ዓ."`.
    #[must_use]
    pub const fn amharic_abbrev(self) -> &'static str {
        match self {
            Self::AmeteMihret => "ዓ.ም.",
            Self::AmeteAlem => "ዓ.ዓ.",
        }
    }

    /// Latin abbreviation: `"E.C."` (Ethiopian Calendar) or `"A.A."`.
    #[must_use]
    pub const fn latin_abbrev(self) -> &'static str {
        match self {
            Self::AmeteMihret => "E.C.",
            Self::AmeteAlem => "A.A.",
        }
    }

    /// Full Amharic name.
    #[must_use]
    pub const fn amharic(self) -> &'static str {
        match self {
            Self::AmeteMihret => "ዓመተ ምሕረት",
            Self::AmeteAlem => "ዓመተ ዓለም",
        }
    }

    /// Full English name.
    #[must_use]
    pub const fn english(self) -> &'static str {
        match self {
            Self::AmeteMihret => "Amete Mihret",
            Self::AmeteAlem => "Amete Alem",
        }
    }

    /// Parses an era from a short tag: `am`/`ec`/`mihret` or `aa`/`alem`.
    pub fn parse(s: &str) -> Result<Self> {
        // Deliberately ASCII-case-insensitive and punctuation tolerant so that
        // `--era A.M.` and `--era am` both work in the CLI.
        let mut buf = [0u8; 16];
        let mut n = 0;
        for b in s.bytes() {
            if b.is_ascii_alphabetic() && n < buf.len() {
                buf[n] = b.to_ascii_lowercase();
                n += 1;
            }
        }
        match &buf[..n] {
            b"am" | b"ec" | b"mihret" | b"ametemihret" => Ok(Self::AmeteMihret),
            b"aa" | b"alem" | b"ametealem" => Ok(Self::AmeteAlem),
            _ => Err(Error::Parse(crate::error::ParseError::Shape)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_is_exactly_5500() {
        assert_eq!(Era::AmeteAlem.from_amete_mihret(2017), 7517);
        assert_eq!(Era::AmeteAlem.to_amete_mihret(7517).unwrap(), 2017);
        assert_eq!(Era::AmeteMihret.from_amete_mihret(2017), 2017);
    }

    #[test]
    fn roundtrip_across_zero() {
        for y in [-5500, -1, 0, 1, 2017, 9999] {
            let aa = Era::AmeteAlem.from_amete_mihret(y);
            assert_eq!(Era::AmeteAlem.to_amete_mihret(aa as i32).unwrap(), y);
        }
    }

    #[test]
    fn parsing() {
        assert_eq!(Era::parse("am").unwrap(), Era::AmeteMihret);
        assert_eq!(Era::parse("E.C.").unwrap(), Era::AmeteMihret);
        assert_eq!(Era::parse("AA").unwrap(), Era::AmeteAlem);
        assert_eq!(Era::parse("amete alem").unwrap(), Era::AmeteAlem);
        assert!(Era::parse("byzantine").is_err());
    }

    #[test]
    fn out_of_range_is_an_error() {
        assert!(Era::AmeteAlem.to_amete_mihret(i32::MIN).is_err());
    }
}
