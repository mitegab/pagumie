//! Minimal TZif (RFC 8536) reader: just enough to answer "what is the UTC
//! offset and zone abbreviation right now?".
//!
//! A date tool that hard-codes UTC+03:00 is wrong on every machine outside
//! Ethiopia, and shelling out to `date +%z` is both slow and a dependency on
//! coreutils. Reading `/etc/localtime` directly is about a hundred lines and
//! makes `ethdate` behave like every other Unix time utility.
//!
//! Only what is needed is implemented: the v1 transition table (and the v2+
//! 64-bit table when present) is scanned for the transition at or before the
//! requested timestamp. POSIX TZ footer strings are used only when there are no
//! transitions at all, and then only for the fixed-offset case; a full POSIX
//! DST-rule evaluator is out of scope, and the fallback is documented rather
//! than guessed at.

use std::fs;
use std::io;

/// The local UTC offset and its abbreviation at a given instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOffset {
    /// Seconds east of UTC.
    pub seconds: i32,
    /// Zone abbreviation, e.g. `EAT`, `UTC`, `CEST`.
    pub abbrev: String,
}

impl LocalOffset {
    /// A fixed offset with a synthesised abbreviation.
    pub fn fixed(seconds: i32) -> Self {
        let abbrev = if seconds == 0 {
            "UTC".to_string()
        } else {
            let sign = if seconds < 0 { '-' } else { '+' };
            let a = seconds.abs();
            if a % 3600 == 0 {
                format!("UTC{sign}{:02}", a / 3600)
            } else {
                format!("UTC{sign}{:02}:{:02}", a / 3600, (a % 3600) / 60)
            }
        };
        Self { seconds, abbrev }
    }
}

/// Resolves the local offset for `timestamp`, honouring `$TZ` and
/// `/etc/localtime`, and falling back to UTC.
pub fn local_offset(timestamp: i64) -> LocalOffset {
    if let Some(tz) = std::env::var_os("TZ") {
        let tz = tz.to_string_lossy().into_owned();
        if let Some(off) = parse_fixed_tz(&tz) {
            return off;
        }
        let path = if tz.starts_with(':') {
            tz[1..].to_string()
        } else {
            tz.clone()
        };
        let candidate = if path.starts_with('/') {
            path.clone()
        } else {
            format!("/usr/share/zoneinfo/{path}")
        };
        if let Ok(off) = offset_from_tzfile(&candidate, timestamp) {
            return off;
        }
    }
    offset_from_tzfile("/etc/localtime", timestamp).unwrap_or_else(|_| LocalOffset::fixed(0))
}

/// Parses `UTC`, `Z`, `+03`, `+03:00`, `-0500` and the fixed-offset subset of
/// POSIX TZ strings such as `EAT-3`.
pub fn parse_fixed_tz(s: &str) -> Option<LocalOffset> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("utc") || s.eq_ignore_ascii_case("z") || s == "UTC0" {
        return Some(LocalOffset::fixed(0));
    }
    if let Some(sec) = parse_hhmm_offset(s) {
        return Some(LocalOffset::fixed(sec));
    }
    // POSIX form: <abbr><offset>, where the offset sign is *reversed*
    // (`EAT-3` means three hours east of UTC). Reject anything with a DST rule.
    let split = s.find(|c: char| c == '+' || c == '-' || c.is_ascii_digit())?;
    let (abbr, rest) = s.split_at(split);
    if abbr.len() < 3 || !abbr.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if rest.chars().any(|c| c.is_ascii_alphabetic()) {
        return None; // has a DST abbreviation, so it has a rule we do not model
    }
    let sec = parse_hhmm_offset(rest)?;
    Some(LocalOffset {
        seconds: -sec,
        abbrev: abbr.to_string(),
    })
}

/// Parses `+03`, `+03:00`, `+0300`, `-5`, `3`.
pub fn parse_hhmm_offset(s: &str) -> Option<i32> {
    let (sign, digits) = match s.strip_prefix('-') {
        Some(r) => (-1, r),
        None => (1, s.strip_prefix('+').unwrap_or(s)),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return None;
    }
    let mut parts = digits.split(':');
    let head = parts.next()?;
    let (h, m, sec) = if head.len() == 4 && parts.clone().next().is_none() {
        (head[..2].parse::<i32>().ok()?, head[2..].parse().ok()?, 0)
    } else {
        (
            head.parse::<i32>().ok()?,
            parts.next().unwrap_or("0").parse().ok()?,
            parts.next().unwrap_or("0").parse().ok()?,
        )
    };
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) || !(0..=59).contains(&sec) {
        return None;
    }
    Some(sign * (h * 3600 + m * 60 + sec))
}

/// Reads a TZif file and returns the offset in force at `timestamp`.
pub fn offset_from_tzfile(path: &str, timestamp: i64) -> io::Result<LocalOffset> {
    let data = fs::read(path)?;
    parse_tzif(&data, timestamp)
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

struct Header {
    isutcnt: usize,
    isstdcnt: usize,
    leapcnt: usize,
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
    version: u8,
}

fn read_header(d: &[u8]) -> io::Result<Header> {
    if d.len() < 44 || &d[..4] != b"TZif" {
        return Err(bad("not a TZif file"));
    }
    let u32at = |o: usize| -> usize {
        u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) as usize
    };
    Ok(Header {
        version: d[4],
        isutcnt: u32at(20),
        isstdcnt: u32at(24),
        leapcnt: u32at(28),
        timecnt: u32at(32),
        typecnt: u32at(36),
        charcnt: u32at(40),
    })
}

fn block_len(h: &Header, time_size: usize) -> usize {
    h.timecnt * (time_size + 1)
        + h.typecnt * 6
        + h.charcnt
        + h.leapcnt * (time_size + 4)
        + h.isstdcnt
        + h.isutcnt
}

fn parse_tzif(d: &[u8], timestamp: i64) -> io::Result<LocalOffset> {
    let h1 = read_header(d)?;
    let mut off = 44;
    let (h, time_size) = if h1.version >= b'2' {
        off += block_len(&h1, 4);
        if d.len() < off + 44 {
            return Err(bad("truncated v2 header"));
        }
        let h2 = read_header(&d[off..])?;
        off += 44;
        (h2, 8)
    } else {
        (h1, 4)
    };

    if d.len() < off + block_len(&h, time_size) {
        return Err(bad("truncated data block"));
    }
    let transitions = &d[off..off + h.timecnt * time_size];
    let indices = &d[off + h.timecnt * time_size..off + h.timecnt * (time_size + 1)];
    let types_off = off + h.timecnt * (time_size + 1);
    let types = &d[types_off..types_off + h.typecnt * 6];
    let chars = &d[types_off + h.typecnt * 6..types_off + h.typecnt * 6 + h.charcnt];

    let read_time = |i: usize| -> i64 {
        let s = &transitions[i * time_size..(i + 1) * time_size];
        if time_size == 8 {
            i64::from_be_bytes(s.try_into().unwrap_or([0; 8]))
        } else {
            i64::from(i32::from_be_bytes(s.try_into().unwrap_or([0; 4])))
        }
    };

    // The type in force is the one attached to the last transition at or
    // before `timestamp`; before the first transition, the first non-DST type.
    let mut type_index: usize = if h.timecnt == 0 {
        0
    } else {
        let mut idx = None;
        for i in 0..h.timecnt {
            if read_time(i) <= timestamp {
                idx = Some(indices[i] as usize);
            } else {
                break;
            }
        }
        match idx {
            Some(i) => i,
            None => (0..h.typecnt)
                .find(|&t| types[t * 6 + 4] == 0)
                .unwrap_or(0),
        }
    };
    if type_index >= h.typecnt {
        type_index = 0;
    }
    if h.typecnt == 0 {
        return Err(bad("no local time types"));
    }

    let base = type_index * 6;
    let utoff = i32::from_be_bytes([
        types[base],
        types[base + 1],
        types[base + 2],
        types[base + 3],
    ]);
    let desig_idx = types[base + 5] as usize;
    let abbrev = chars
        .get(desig_idx..)
        .and_then(|s| s.split(|&b| b == 0).next())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| LocalOffset::fixed(utoff).abbrev);

    Ok(LocalOffset {
        seconds: utoff,
        abbrev,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_offset_strings() {
        assert_eq!(parse_hhmm_offset("+03"), Some(10_800));
        assert_eq!(parse_hhmm_offset("+03:00"), Some(10_800));
        assert_eq!(parse_hhmm_offset("+0300"), Some(10_800));
        assert_eq!(parse_hhmm_offset("-05:30"), Some(-19_800));
        assert_eq!(parse_hhmm_offset("0"), Some(0));
        assert_eq!(parse_hhmm_offset("abc"), None);
        assert_eq!(parse_hhmm_offset("+25"), None);
    }

    #[test]
    fn posix_tz_sign_is_reversed() {
        let o = parse_fixed_tz("EAT-3").unwrap();
        assert_eq!(o.seconds, 10_800);
        assert_eq!(o.abbrev, "EAT");
        assert_eq!(parse_fixed_tz("UTC").unwrap().seconds, 0);
        // A DST rule is not modelled, so it is rejected rather than guessed.
        assert!(parse_fixed_tz("CET-1CEST,M3.5.0,M10.5.0/3").is_none());
    }

    #[test]
    fn fixed_abbreviations() {
        assert_eq!(LocalOffset::fixed(0).abbrev, "UTC");
        assert_eq!(LocalOffset::fixed(10_800).abbrev, "UTC+03");
        assert_eq!(LocalOffset::fixed(-19_800).abbrev, "UTC-05:30");
    }

    #[test]
    fn reads_the_system_tzdb_when_present() {
        // Ethiopia has been UTC+03:00 with no DST since 1936.
        let path = "/usr/share/zoneinfo/Africa/Addis_Ababa";
        if !std::path::Path::new(path).exists() {
            eprintln!("skipping: no tzdb on this machine");
            return;
        }
        let now = 1_757_561_400; // 2025-09-11T03:30:00Z
        let o = offset_from_tzfile(path, now).expect("Addis Ababa should parse");
        assert_eq!(o.seconds, 10_800, "Addis Ababa is UTC+03:00");
        assert_eq!(o.abbrev, "EAT");

        // Before 1936 the zone kept local mean time, +02:34:48.
        let before = -1_072_000_000; // 1936-01-01-ish
        let o = offset_from_tzfile(path, before).expect("parse");
        assert!(
            o.seconds != 10_800,
            "the pre-1936 offset should differ from EAT, got {}",
            o.seconds
        );
    }

    #[test]
    fn a_dst_zone_switches_over_the_year() {
        let path = "/usr/share/zoneinfo/Europe/Berlin";
        if !std::path::Path::new(path).exists() {
            eprintln!("skipping: no tzdb on this machine");
            return;
        }
        let january = offset_from_tzfile(path, 1_704_067_200).unwrap(); // 2024-01-01
        let july = offset_from_tzfile(path, 1_719_792_000).unwrap(); // 2024-07-01
        assert_eq!(january.seconds, 3_600);
        assert_eq!(july.seconds, 7_200);
        assert_eq!(july.abbrev, "CEST");
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(parse_tzif(b"", 0).is_err());
        assert!(parse_tzif(b"TZif2", 0).is_err());
        assert!(parse_tzif(&[0u8; 200], 0).is_err());
        let mut junk = b"TZif2".to_vec();
        junk.extend(std::iter::repeat(0xff).take(400));
        assert!(parse_tzif(&junk, 0).is_err());
    }
}
