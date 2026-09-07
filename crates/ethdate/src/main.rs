//! `ethdate` — the Ethiopian calendar as a Unix command-line tool.
//!
//! Design notes, since they are the difference between a toy and something that
//! belongs in `/usr/bin`:
//!
//! * It reads standard input and writes standard output, one record per line,
//!   so it composes with `xargs`, `sed`, `jq` and friends. `--null` switches to
//!   NUL-separated records for filenames and other hostile data.
//! * Exit status is meaningful: 0 success, 1 a date could not be parsed or
//!   converted, 2 the command line was wrong. `--is-holiday` uses 0/1 as a
//!   predicate, like `test(1)`.
//! * A broken pipe is not an error: `ethdate --holidays | head -1` exits 0 and
//!   says nothing, as every other Unix filter does.
//! * Errors go to standard error, prefixed with the program name and the input
//!   that caused them, and never interleave with the data stream.
//! * No runtime dependencies at all, so the binary is small and the supply
//!   chain is the standard library.


mod args;
mod cal;
mod render;
mod tz;

use std::io::{BufRead, IsTerminal, Write};

use ethiopic_calendar::clock::EthiopianDateTime;
use ethiopic_calendar::holidays::holidays_on;
use ethiopic_calendar::{EthiopianDate, GregorianDate, JulianDate, RataDie};

use args::{Action, Calendar, Options, Zone, EXIT_FAILURE, EXIT_OK, EXIT_USAGE, PROGRAM};
use render::Resolved;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = match args::parse(argv) {
        Ok(options) => run(&options),
        Err(e) => {
            eprintln!("{PROGRAM}: {e}");
            eprintln!("Try '{PROGRAM} --help' for more information.");
            EXIT_USAGE
        }
    };
    std::process::exit(code);
}

/// Current Unix time; the epoch itself if the clock is before 1970 and the
/// subtraction fails, which is good enough for a date tool.
fn now_timestamp() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

fn run(o: &Options) -> i32 {
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let code = dispatch(o, &mut out);
    if let Err(e) = out.flush() {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            return EXIT_OK;
        }
        if !o.quiet {
            eprintln!("{PROGRAM}: write error: {e}");
        }
        return EXIT_FAILURE;
    }
    code
}

#[allow(clippy::too_many_lines)] // One flat dispatch reads better than five hops.
fn dispatch<W: Write>(o: &Options, out: &mut W) -> i32 {
    let now = now_timestamp();
    let zone = match o.zone {
        Zone::Local => tz::local_offset(now),
        Zone::Utc => tz::LocalOffset::fixed(0),
        Zone::Fixed(s) => tz::LocalOffset::fixed(s),
    };

    match o.action {
        Action::Help => {
            let _ = out.write_all(args::HELP.as_bytes());
            return EXIT_OK;
        }
        Action::Version => {
            let _ = writeln!(
                out,
                "{PROGRAM} {}\nethiopic-calendar {}\n{}",
                env!("CARGO_PKG_VERSION"),
                ethiopic_calendar::VERSION,
                "Licensed under MIT or Apache-2.0."
            );
            return EXIT_OK;
        }
        _ => {}
    }

    let today = match EthiopianDateTime::from_unix_timestamp(now, zone.seconds, o.day_start) {
        Ok(dt) => dt,
        Err(e) => {
            if !o.quiet {
                eprintln!("{PROGRAM}: cannot read the system clock: {e}");
            }
            return EXIT_FAILURE;
        }
    };

    match o.action {
        Action::Calendar | Action::YearCalendar => {
            return calendar(o, out, today.date());
        }
        Action::Holidays => {
            let year = o.year.unwrap_or_else(|| today.date().year());
            return match render::holidays_table(year, o) {
                Ok(t) => {
                    let _ = writeln!(out, "{t}");
                    EXIT_OK
                }
                Err(e) => fail(o, &format!("{year}"), &e.to_string()),
            };
        }
        Action::Computus => {
            let year = o.year.unwrap_or_else(|| today.date().year());
            return match render::computus_table(year, o) {
                Ok(t) => {
                    let _ = writeln!(out, "{t}");
                    EXIT_OK
                }
                Err(e) => fail(o, &format!("{year}"), &e.to_string()),
            };
        }
        _ => {}
    }

    // Gather the inputs: positionals, then standard input if asked for. With
    // neither, the answer is "now", like date(1).
    let mut inputs: Vec<String> = o.inputs.clone();
    if o.stdin {
        let sep = if o.null { b'\0' } else { b'\n' };
        let stdin = std::io::stdin();
        let mut buf = Vec::new();
        let mut locked = stdin.lock();
        loop {
            buf.clear();
            match locked.read_until(sep, &mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    while buf.last().is_some_and(|&b| b == sep || b == b'\r') {
                        buf.pop();
                    }
                    if buf.is_empty() {
                        continue;
                    }
                    match std::str::from_utf8(&buf) {
                        Ok(s) => inputs.push(s.to_string()),
                        Err(_) => {
                            if !o.quiet {
                                eprintln!("{PROGRAM}: standard input is not valid UTF-8");
                            }
                            return EXIT_FAILURE;
                        }
                    }
                }
                Err(e) => {
                    if !o.quiet {
                        eprintln!("{PROGRAM}: cannot read standard input: {e}");
                    }
                    return EXIT_FAILURE;
                }
            }
        }
    }
    if inputs.is_empty() {
        inputs.push("now".to_string());
    }

    if o.action == Action::IsHoliday {
        let input = &inputs[0];
        return match resolve(input, o, &today, zone.seconds) {
            Ok(r) => match holidays_on(r.date).next() {
                Some(h) => {
                    if !o.quiet {
                        let _ = writeln!(
                            out,
                            "{}",
                            match o.locale {
                                ethiopic_calendar::format::Locale::Amharic => h.amharic(),
                                ethiopic_calendar::format::Locale::Latin => h.english(),
                            }
                        );
                    }
                    EXIT_OK
                }
                None => EXIT_FAILURE,
            },
            Err(e) => fail(o, input, &e),
        };
    }

    let terminator = if o.null { b'\0' } else { b'\n' };
    let mut status = EXIT_OK;
    for input in &inputs {
        match resolve(input, o, &today, zone.seconds) {
            Ok(r) => match render::render(&r, o) {
                Ok(text) => {
                    if out.write_all(text.as_bytes()).is_err()
                        || out.write_all(&[terminator]).is_err()
                    {
                        return EXIT_OK; // broken pipe
                    }
                }
                Err(e) => status = fail(o, input, &e.to_string()),
            },
            Err(e) => status = fail(o, input, &e),
        }
    }
    status
}

fn calendar<W: Write>(o: &Options, out: &mut W, today: EthiopianDate) -> i32 {
    let style = cal::Style {
        locale: o.locale,
        color: !o.no_color && std::io::stdout().is_terminal(),
        today: Some(today),
    };
    let year = o.year.unwrap_or_else(|| today.year());

    // `ethdate --calendar 2018` prints the whole year, as cal(1) does; a month
    // is selected either by --calendar YYYY-MM or by defaulting to today's.
    let month = o.inputs.first().and_then(|s| parse_year_month(s));
    let whole_year = o.action == Action::YearCalendar || (o.year.is_some() && month.is_none());

    let rendered = if whole_year {
        cal::year(year, style)
    } else {
        let (y, m) = month.unwrap_or((today.year(), today.month()));
        cal::month(y, m, style)
    };
    match rendered {
        Ok(text) => {
            let _ = writeln!(out, "{text}");
            EXIT_OK
        }
        Err(e) => fail(o, &year.to_string(), &e.to_string()),
    }
}

/// Accepts `2018-01` and `2018-01-01`, returning the year and month.
fn parse_year_month(s: &str) -> Option<(i32, u8)> {
    let mut parts = s.split('-');
    let y = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<u8>().ok()?;
    if (1..=13).contains(&m) {
        Some((y, m))
    } else {
        None
    }
}

fn fail(o: &Options, input: &str, message: &str) -> i32 {
    if !o.quiet {
        eprintln!("{PROGRAM}: {input}: {message}");
    }
    EXIT_FAILURE
}

/// Turns one input string into a date, honouring `--from`.
fn resolve(
    input: &str,
    o: &Options,
    today: &EthiopianDateTime,
    offset: i32,
) -> Result<Resolved, String> {
    let s = input.trim();
    let shift = |days: i64| -> Result<Resolved, String> {
        today
            .date()
            .add_days(days)
            .map(|date| Resolved {
                date,
                datetime: None,
            })
            .map_err(|e| e.to_string())
    };

    match s.to_ascii_lowercase().as_str() {
        "now" | "today" => {
            return Ok(Resolved {
                date: today.date(),
                datetime: Some(*today),
            })
        }
        "yesterday" => return shift(-1),
        "tomorrow" => return shift(1),
        _ => {}
    }

    if let Some(ts) = s.strip_prefix('@') {
        let secs: i64 = ts
            .parse()
            .map_err(|_| format!("{ts:?} is not a Unix timestamp"))?;
        return from_timestamp(secs, o, offset);
    }
    // A leading sign with no other '-' is a relative day count: +30, -7.
    if (s.starts_with('+') || s.starts_with('-'))
        && s[1..].chars().all(|c| c.is_ascii_digit())
        && s.len() > 1
    {
        let n: i64 = s.parse().map_err(|_| format!("{s:?} is not a day count"))?;
        return shift(n);
    }

    let date = match o.from {
        Calendar::Ethiopic => s
            .parse::<EthiopianDate>()
            .map_err(|e| e.to_string())?,
        Calendar::Gregorian => {
            let g = s.parse::<GregorianDate>().map_err(|e| e.to_string())?;
            EthiopianDate::from_gregorian(g).map_err(|e| e.to_string())?
        }
        Calendar::Julian => {
            let j = s.parse::<JulianDate>().map_err(|e| e.to_string())?;
            EthiopianDate::from_julian(j).map_err(|e| e.to_string())?
        }
        Calendar::Jdn => {
            let n: i64 = s.parse().map_err(|_| format!("{s:?} is not a number"))?;
            EthiopianDate::from_jdn(n).map_err(|e| e.to_string())?
        }
        Calendar::RataDie => {
            let n: i64 = s.parse().map_err(|_| format!("{s:?} is not a number"))?;
            EthiopianDate::from_rata_die(RataDie::new(n)).map_err(|e| e.to_string())?
        }
        Calendar::Unix => {
            let n: i64 = s.parse().map_err(|_| format!("{s:?} is not a number"))?;
            return from_timestamp(n, o, offset);
        }
    };
    Ok(Resolved {
        date,
        datetime: None,
    })
}

fn from_timestamp(secs: i64, o: &Options, offset: i32) -> Result<Resolved, String> {
    let dt = EthiopianDateTime::from_unix_timestamp(secs, offset, o.day_start)
        .map_err(|e| e.to_string())?;
    Ok(Resolved {
        date: dt.date(),
        datetime: Some(dt),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use args::Output;
    use ethiopic_calendar::clock::DayStart;

    fn opts() -> Options {
        Options {
            locale: ethiopic_calendar::format::Locale::Latin,
            ..Options::default()
        }
    }

    fn today() -> EthiopianDateTime {
        // 2025-09-11T09:30:00+03:00 = 1 Meskerem 2018, 3:30 in the morning.
        EthiopianDateTime::from_unix_timestamp(1_757_572_200, 10_800, DayStart::Dawn).unwrap()
    }

    #[test]
    fn keywords_resolve() {
        let o = opts();
        let t = today();
        assert_eq!(
            resolve("today", &o, &t, 10_800).unwrap().date.as_tuple(),
            (2018, 1, 1)
        );
        assert_eq!(
            resolve("yesterday", &o, &t, 10_800).unwrap().date.as_tuple(),
            (2017, 13, 5)
        );
        assert_eq!(
            resolve("tomorrow", &o, &t, 10_800).unwrap().date.as_tuple(),
            (2018, 1, 2)
        );
        assert!(resolve("now", &o, &t, 10_800).unwrap().datetime.is_some());
    }

    #[test]
    fn relative_day_counts() {
        let o = opts();
        let t = today();
        assert_eq!(
            resolve("+30", &o, &t, 10_800).unwrap().date.as_tuple(),
            (2018, 2, 1)
        );
        assert_eq!(
            resolve("-1", &o, &t, 10_800).unwrap().date.as_tuple(),
            (2017, 13, 5)
        );
    }

    #[test]
    fn timestamps_carry_a_time_of_day() {
        let o = opts();
        let r = resolve("@1757572200", &o, &today(), 10_800).unwrap();
        let dt = r.datetime.expect("a timestamp has a time");
        assert_eq!(dt.time().ethiopian_hour(), 3);
        assert_eq!(dt.time().minute(), 30);
    }

    #[test]
    fn from_other_calendars() {
        let t = today();
        let mut o = opts();
        o.from = Calendar::Gregorian;
        assert_eq!(
            resolve("2024-09-11", &o, &t, 0).unwrap().date.as_tuple(),
            (2017, 1, 1)
        );
        o.from = Calendar::Julian;
        assert_eq!(
            resolve("2024-08-29", &o, &t, 0).unwrap().date.as_tuple(),
            (2017, 1, 1)
        );
        o.from = Calendar::Jdn;
        assert_eq!(
            resolve("2460565", &o, &t, 0).unwrap().date.as_tuple(),
            (2017, 1, 1)
        );
        o.from = Calendar::RataDie;
        assert_eq!(
            resolve("739140", &o, &t, 0).unwrap().date.as_tuple(),
            (2017, 1, 1)
        );
    }

    #[test]
    fn bad_input_is_reported_not_panicked() {
        let o = opts();
        let t = today();
        for bad in ["", "not-a-date", "@x", "2017-99-99", "2017-13-07"] {
            assert!(resolve(bad, &o, &t, 0).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn year_month_parsing() {
        assert_eq!(parse_year_month("2018-01"), Some((2018, 1)));
        assert_eq!(parse_year_month("2018-13-06"), Some((2018, 13)));
        assert_eq!(parse_year_month("2018"), None);
        assert_eq!(parse_year_month("2018-14"), None);
    }

    #[test]
    fn output_is_written_for_every_input() {
        let mut o = opts();
        o.inputs = vec!["2017-01-01".into(), "2017-01-02".into()];
        o.output = Output::Iso;
        let mut buf = Vec::new();
        assert_eq!(dispatch(&o, &mut buf), EXIT_OK);
        assert_eq!(String::from_utf8(buf).unwrap(), "2017-01-01\n2017-01-02\n");
    }

    #[test]
    fn one_bad_input_fails_the_run_but_not_the_others() {
        let mut o = opts();
        o.inputs = vec!["2017-01-01".into(), "rubbish".into(), "2017-01-03".into()];
        o.output = Output::Iso;
        o.quiet = true;
        let mut buf = Vec::new();
        assert_eq!(dispatch(&o, &mut buf), EXIT_FAILURE);
        assert_eq!(String::from_utf8(buf).unwrap(), "2017-01-01\n2017-01-03\n");
    }

    #[test]
    fn null_separated_output() {
        let mut o = opts();
        o.inputs = vec!["2017-01-01".into(), "2017-01-02".into()];
        o.output = Output::Iso;
        o.null = true;
        let mut buf = Vec::new();
        dispatch(&o, &mut buf);
        assert_eq!(buf, b"2017-01-01\x002017-01-02\x00");
    }

    #[test]
    fn is_holiday_is_a_predicate() {
        let mut o = opts();
        o.action = Action::IsHoliday;
        o.quiet = true;
        o.inputs = vec!["2017-01-01".into()]; // Enkutatash
        assert_eq!(dispatch(&o, &mut Vec::new()), EXIT_OK);
        o.inputs = vec!["2017-01-02".into()];
        assert_eq!(dispatch(&o, &mut Vec::new()), EXIT_FAILURE);
    }

    #[test]
    fn help_and_version_succeed() {
        let mut o = opts();
        o.action = Action::Help;
        let mut buf = Vec::new();
        assert_eq!(dispatch(&o, &mut buf), EXIT_OK);
        assert!(String::from_utf8(buf).unwrap().contains("USAGE"));
        o.action = Action::Version;
        let mut buf = Vec::new();
        assert_eq!(dispatch(&o, &mut buf), EXIT_OK);
        assert!(String::from_utf8(buf).unwrap().contains(env!("CARGO_PKG_VERSION")));
    }
}
