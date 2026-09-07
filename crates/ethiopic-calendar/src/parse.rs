//! Shared, allocation-free `YYYY-MM-DD` parsing.
//!
//! Hand-rolled rather than pulled from a dependency because the crate is
//! `no_std` + zero-dependency by design, and because the leading-minus case
//! (proleptic negative years) is exactly the kind of thing a generic parser
//! gets wrong.

use crate::error::{Error, ParseError, Result};

/// Parses `[-]Y[Y...]-M[M]-D[D]` into `(year, month, day)` as `i64`s.
///
/// Field ranges are *not* validated here; each calendar applies its own rules.
pub(crate) fn parse_ymd(s: &str) -> Result<(i64, i64, i64)> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::Parse(ParseError::Empty));
    }
    let (negative, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mut parts = rest.split('-');
    let y = parts.next().ok_or(Error::Parse(ParseError::Shape))?;
    let m = parts.next().ok_or(Error::Parse(ParseError::Shape))?;
    let d = parts.next().ok_or(Error::Parse(ParseError::Shape))?;
    if parts.next().is_some() {
        return Err(Error::Parse(ParseError::Trailing));
    }
    let year = parse_u(y)?;
    let month = parse_u(m)?;
    let day = parse_u(d)?;
    Ok((if negative { -year } else { year }, month, day))
}

fn parse_u(s: &str) -> Result<i64> {
    if s.is_empty() {
        return Err(Error::Parse(ParseError::Shape));
    }
    let mut acc: i64 = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return Err(Error::Parse(ParseError::NotANumber));
        }
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(i64::from(b - b'0')))
            .ok_or(Error::Overflow)?;
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(parse_ymd("2017-01-01").unwrap(), (2017, 1, 1));
        assert_eq!(parse_ymd("2017-1-1").unwrap(), (2017, 1, 1));
        assert_eq!(parse_ymd(" 2017-13-06 ").unwrap(), (2017, 13, 6));
        assert_eq!(parse_ymd("-0044-03-15").unwrap(), (-44, 3, 15));
        assert_eq!(parse_ymd("+2017-01-01").unwrap(), (2017, 1, 1));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_ymd("").is_err());
        assert!(parse_ymd("2017").is_err());
        assert!(parse_ymd("2017-01").is_err());
        assert!(parse_ymd("2017-01-01-01").is_err());
        assert!(parse_ymd("20a7-01-01").is_err());
        assert!(parse_ymd("2017--1-01").is_err());
    }
}
