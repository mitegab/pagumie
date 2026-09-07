//! Command-line argument parsing.
//!
//! Hand-rolled: the tool has no runtime dependencies, so `clap` is out, and the
//! grammar is small enough that a hand parser is shorter than the derive
//! attributes would be. It follows the usual GNU conventions — long options
//! with `--opt value` or `--opt=value`, short options that may be bundled,
//! `--` to stop option parsing, `-` to mean standard input.

use std::fmt;

use ethiopic_calendar::clock::DayStart;
use ethiopic_calendar::era::Era;
use ethiopic_calendar::format::Locale;

/// Program name as it appears in messages.
pub const PROGRAM: &str = "ethdate";

/// Exit status: success.
pub const EXIT_OK: i32 = 0;
/// Exit status: a date could not be parsed or converted.
pub const EXIT_FAILURE: i32 = 1;
/// Exit status: the command line itself was wrong.
pub const EXIT_USAGE: i32 = 2;

/// Which calendar an input is read in, or an output written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Calendar {
    /// Ethiopian, Amete Mihret.
    #[default]
    Ethiopic,
    /// Proleptic Gregorian.
    Gregorian,
    /// Julian (no year 0).
    Julian,
    /// Julian Day Number.
    Jdn,
    /// Fixed day number (R.D.).
    RataDie,
    /// Unix timestamp in seconds.
    Unix,
}

impl Calendar {
    fn parse(s: &str) -> Result<Self, UsageError> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "ethiopic" | "ethiopian" | "eth" | "ec" | "et" => Self::Ethiopic,
            "gregorian" | "greg" | "g" | "gc" => Self::Gregorian,
            "julian" | "jul" | "j" => Self::Julian,
            "jdn" | "jd" => Self::Jdn,
            "rd" | "ratadie" | "rata-die" | "fixed" => Self::RataDie,
            "unix" | "epoch" | "posix" => Self::Unix,
            other => {
                return Err(UsageError(format!(
                    "unknown calendar {other:?} (want ethiopic, gregorian, julian, jdn, rd or unix)"
                )))
            }
        })
    }
}

/// How the result is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Output {
    /// Human-readable, locale-aware.
    #[default]
    Text,
    /// `%Y-%m-%d` and nothing else.
    Iso,
    /// One JSON object per input.
    Json,
    /// A single JSON object shaped for Waybar's `custom` module.
    Waybar,
    /// One line with polybar/i3blocks formatting.
    Polybar,
}

/// Which timezone offset to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zone {
    /// Read `$TZ`, else `/etc/localtime`.
    #[default]
    Local,
    /// Force UTC.
    Utc,
    /// A fixed offset in seconds east of UTC.
    Fixed(i32),
}

/// What the program should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Action {
    /// Convert and print dates (the default).
    #[default]
    Convert,
    /// Print a month grid.
    Calendar,
    /// Print a whole Ethiopian year of month grids.
    YearCalendar,
    /// List the holidays of a year.
    Holidays,
    /// Print the Baḥire Ḥasab table of a year.
    Computus,
    /// Exit 0 if the date is a public holiday, 1 if not.
    IsHoliday,
    /// Print usage and exit 0.
    Help,
    /// Print the version and exit 0.
    Version,
}

/// A fully parsed command line.
#[derive(Debug, Clone)]
pub struct Options {
    /// Dates to convert; empty means "now".
    pub inputs: Vec<String>,
    /// `--format`.
    pub format: Option<String>,
    /// `--from`.
    pub from: Calendar,
    /// `--to`.
    pub to: Calendar,
    /// Output shape.
    pub output: Output,
    /// Script for names.
    pub locale: Locale,
    /// Era used when printing years.
    pub era: Era,
    /// Whether a day turns at dawn or midnight.
    pub day_start: DayStart,
    /// Time zone.
    pub zone: Zone,
    /// Include the time of day in the default output.
    pub with_time: bool,
    /// What to do.
    pub action: Action,
    /// Year argument for `--calendar`, `--holidays`, `--computus`.
    pub year: Option<i32>,
    /// Read dates from standard input.
    pub stdin: bool,
    /// Use NUL as the record separator instead of newline.
    pub null: bool,
    /// Suppress error messages.
    pub quiet: bool,
    /// Never emit ANSI colour, even on a terminal.
    pub no_color: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            format: None,
            from: Calendar::Ethiopic,
            to: Calendar::Ethiopic,
            output: Output::Text,
            locale: default_locale(),
            era: Era::AmeteMihret,
            day_start: DayStart::Dawn,
            zone: Zone::Local,
            with_time: false,
            action: Action::Convert,
            year: None,
            stdin: false,
            null: false,
            quiet: false,
            no_color: std::env::var_os("NO_COLOR").is_some(),
        }
    }
}

/// Amharic unless the environment asks for something else, since the tool's
/// primary audience reads Amharic; `LC_ALL`/`LC_TIME`/`LANG` win when set.
fn default_locale() -> Locale {
    for key in ["LC_ALL", "LC_TIME", "LANG"] {
        if let Some(v) = std::env::var_os(key) {
            let v = v.to_string_lossy().to_ascii_lowercase();
            if v.is_empty() {
                continue;
            }
            if v.starts_with("am") {
                return Locale::Amharic;
            }
            return Locale::Latin;
        }
    }
    Locale::Amharic
}

/// A command line that does not make sense.
#[derive(Debug, Clone)]
pub struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

struct Cursor {
    args: Vec<String>,
    index: usize,
}

impl Cursor {
    fn value(&mut self, flag: &str, inline: Option<String>) -> Result<String, UsageError> {
        if let Some(v) = inline {
            return Ok(v);
        }
        self.index += 1;
        self.args
            .get(self.index)
            .cloned()
            .ok_or_else(|| UsageError(format!("option '{flag}' needs a value")))
    }
}

/// Parses an argument vector (without `argv[0]`).
#[allow(clippy::too_many_lines)] // A flat option table is clearer than a nest of helpers.
pub fn parse(args: Vec<String>) -> Result<Options, UsageError> {
    let mut o = Options::default();
    let mut cur = Cursor { args, index: 0 };
    let mut only_positional = false;
    let mut explicit_to = false;

    while cur.index < cur.args.len() {
        let arg = cur.args[cur.index].clone();
        if only_positional || arg == "-" || !arg.starts_with('-') || arg.len() == 1 {
            if arg == "-" {
                o.stdin = true;
            } else {
                o.inputs.push(arg);
            }
            cur.index += 1;
            continue;
        }
        if arg == "--" {
            only_positional = true;
            cur.index += 1;
            continue;
        }

        if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (long, None),
            };
            match name {
                "help" => o.action = Action::Help,
                "version" => o.action = Action::Version,
                "date" => o.inputs.push(cur.value("--date", inline)?),
                "format" => o.format = Some(cur.value("--format", inline)?),
                "from" => o.from = Calendar::parse(&cur.value("--from", inline)?)?,
                "to" => {
                    o.to = Calendar::parse(&cur.value("--to", inline)?)?;
                    explicit_to = true;
                }
                "gregorian" => {
                    o.to = Calendar::Gregorian;
                    explicit_to = true;
                }
                "ethiopic" | "ethiopian" => {
                    o.to = Calendar::Ethiopic;
                    explicit_to = true;
                }
                "julian" => {
                    o.to = Calendar::Julian;
                    explicit_to = true;
                }
                "jdn" => {
                    o.to = Calendar::Jdn;
                    explicit_to = true;
                }
                "rd" => {
                    o.to = Calendar::RataDie;
                    explicit_to = true;
                }
                "unix" => {
                    o.to = Calendar::Unix;
                    explicit_to = true;
                }
                "iso" => o.output = Output::Iso,
                "json" => o.output = Output::Json,
                "waybar" => o.output = Output::Waybar,
                "polybar" | "i3blocks" | "i3status" => o.output = Output::Polybar,
                "locale" => {
                    o.locale = match cur.value("--locale", inline)?.to_ascii_lowercase().as_str() {
                        "am" | "amharic" | "am_et" | "am-et" | "am_et.utf-8" => Locale::Amharic,
                        "en" | "latin" | "c" | "posix" | "en_us" | "en-us" => Locale::Latin,
                        other => {
                            return Err(UsageError(format!(
                                "unknown locale {other:?} (want 'am' or 'en')"
                            )))
                        }
                    }
                }
                "era" => o.era = Era::parse(&cur.value("--era", inline)?).map_err(to_usage)?,
                "day-start" => {
                    o.day_start = match cur
                        .value("--day-start", inline)?
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "dawn" | "sunrise" | "traditional" | "6" => DayStart::Dawn,
                        "midnight" | "civil" | "0" => DayStart::Midnight,
                        other => {
                            return Err(UsageError(format!(
                                "unknown day start {other:?} (want 'dawn' or 'midnight')"
                            )))
                        }
                    }
                }
                "utc" | "universal" => o.zone = Zone::Utc,
                "offset" => {
                    let raw = cur.value("--offset", inline)?;
                    let secs = crate::tz::parse_hhmm_offset(&raw).ok_or_else(|| {
                        UsageError(format!("cannot read {raw:?} as a UTC offset like +03:00"))
                    })?;
                    o.zone = Zone::Fixed(secs);
                }
                "time" => o.with_time = true,
                "calendar" | "cal" => {
                    o.action = Action::Calendar;
                    if let Some(v) = inline {
                        o.year = Some(parse_year(&v)?);
                    }
                }
                "year" => {
                    o.action = Action::YearCalendar;
                    if let Some(v) = inline {
                        o.year = Some(parse_year(&v)?);
                    }
                }
                "holidays" => {
                    o.action = Action::Holidays;
                    if let Some(v) = inline {
                        o.year = Some(parse_year(&v)?);
                    }
                }
                "computus" | "bahire-hasab" => {
                    o.action = Action::Computus;
                    if let Some(v) = inline {
                        o.year = Some(parse_year(&v)?);
                    }
                }
                "is-holiday" => o.action = Action::IsHoliday,
                "stdin" => o.stdin = true,
                "null" | "zero" => o.null = true,
                "quiet" | "silent" => o.quiet = true,
                "no-color" | "no-colour" => o.no_color = true,
                other => return Err(UsageError(format!("unknown option '--{other}'"))),
            }
            cur.index += 1;
            continue;
        }

        // Short options, possibly bundled: -gq, -f'%Y', -f '%Y'.
        let chars: Vec<char> = arg[1..].chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let rest: String = chars[i + 1..].iter().collect();
            let inline = if rest.is_empty() { None } else { Some(rest) };
            match c {
                'h' => o.action = Action::Help,
                'V' => o.action = Action::Version,
                'q' => o.quiet = true,
                '0' => o.null = true,
                's' => o.stdin = true,
                'T' => o.with_time = true,
                'g' => {
                    o.to = Calendar::Gregorian;
                    explicit_to = true;
                }
                'e' => {
                    o.to = Calendar::Ethiopic;
                    explicit_to = true;
                }
                'j' => o.output = Output::Json,
                'I' => o.output = Output::Iso,
                'c' => o.action = Action::Calendar,
                'y' => o.action = Action::YearCalendar,
                'f' => {
                    o.format = Some(cur.value("-f", inline)?);
                    break;
                }
                'd' => {
                    o.inputs.push(cur.value("-d", inline)?);
                    break;
                }
                'F' => {
                    o.from = Calendar::parse(&cur.value("-F", inline)?)?;
                    break;
                }
                't' => {
                    o.to = Calendar::parse(&cur.value("-t", inline)?)?;
                    explicit_to = true;
                    break;
                }
                'u' => o.zone = Zone::Utc,
                other => return Err(UsageError(format!("unknown option '-{other}'"))),
            }
            i += 1;
        }
        cur.index += 1;
    }

    // `ethdate --calendar 2018` and `ethdate --holidays 2018`: the bare number
    // that follows is a year, not a date to convert.
    if matches!(
        o.action,
        Action::Calendar | Action::YearCalendar | Action::Holidays | Action::Computus
    ) && o.year.is_none()
    {
        if let Some(first) = o.inputs.first() {
            if let Ok(y) = parse_year(first) {
                o.year = Some(y);
                o.inputs.remove(0);
            }
        }
    }

    // Reading Gregorian and printing Gregorian is almost never what was meant;
    // default the other side of the conversion instead of echoing the input.
    if !explicit_to && o.from != Calendar::Ethiopic {
        o.to = Calendar::Ethiopic;
    }
    if !explicit_to && o.from == Calendar::Ethiopic && !o.inputs.is_empty() {
        o.to = Calendar::Ethiopic;
    }

    Ok(o)
}

fn parse_year(s: &str) -> Result<i32, UsageError> {
    s.parse::<i32>()
        .map_err(|_| UsageError(format!("cannot read {s:?} as a year")))
}

fn to_usage(e: ethiopic_calendar::Error) -> UsageError {
    UsageError(e.to_string())
}

/// The `--help` text.
pub const HELP: &str = concat!(
    "ethdate — the Ethiopian calendar on the command line\n",
    "\n",
    "USAGE\n",
    "    ethdate [OPTIONS] [DATE...]\n",
    "\n",
    "    With no DATE, prints today. DATE may be an ISO-shaped date in the\n",
    "    calendar given by --from, one of 'now', 'today', 'yesterday',\n",
    "    'tomorrow', a Unix timestamp as '@1757548800', a relative day count\n",
    "    such as '+30' or '-7', or '-' to read dates from standard input.\n",
    "\n",
    "OPTIONS\n",
    "    -d, --date DATE        convert DATE (may be repeated)\n",
    "    -f, --format FMT       strftime-style format; see FORMAT below\n",
    "    -F, --from CALENDAR    read dates as CALENDAR (default: ethiopic)\n",
    "    -t, --to CALENDAR      print dates as CALENDAR (default: ethiopic)\n",
    "    -g, --gregorian        shorthand for --to gregorian\n",
    "    -e, --ethiopic         shorthand for --to ethiopic\n",
    "        --julian, --jdn, --rd, --unix\n",
    "                           other shorthands for --to\n",
    "    -I, --iso              print bare YYYY-MM-DD\n",
    "    -j, --json             print one JSON object per date\n",
    "        --waybar           print Waybar custom-module JSON\n",
    "        --polybar          print a single line for polybar or i3blocks\n",
    "        --locale am|en     script used for names (default: from $LANG)\n",
    "        --era am|aa        Amete Mihret or Amete Alem year numbering\n",
    "        --day-start dawn|midnight\n",
    "                           when a calendar day turns (default: dawn)\n",
    "    -u, --utc              use UTC instead of the local zone\n",
    "        --offset ±HH:MM    use a fixed UTC offset\n",
    "    -T, --time             include the Ethiopian time of day\n",
    "    -c, --calendar [YEAR]  print a month grid, like cal(1)\n",
    "    -y, --year [YEAR]      print all thirteen months of a year\n",
    "        --holidays [YEAR]  list the holidays of an Ethiopian year\n",
    "        --computus [YEAR]  print the Baḥire Ḥasab table of a year\n",
    "        --is-holiday       exit 0 if the date is a public holiday, else 1\n",
    "    -s, --stdin            read dates from standard input, one per line\n",
    "    -0, --null             separate records with NUL instead of newline\n",
    "    -q, --quiet            no error messages; rely on the exit status\n",
    "        --no-color         never colourise output\n",
    "    -h, --help             this message\n",
    "    -V, --version          version information\n",
    "\n",
    "CALENDARS\n",
    "    ethiopic, gregorian, julian, jdn, rd, unix\n",
    "\n",
    "FORMAT\n",
    "    %Y year   %m month   %d day      %B month name (Amharic)\n",
    "    %A weekday (Amharic)  %a weekday (English)  %j day of year\n",
    "    %G year in Ge'ez numerals        %L Amete Alem year\n",
    "    %E era (E.C.)         %N era (ዓ.ም.)        %g Gregorian date\n",
    "    %J Julian Day Number  %R fixed day number  %s Unix timestamp\n",
    "    %I Ethiopian hour     %H Western hour      %M minute  %S second\n",
    "    %p period (Amharic)   %P period (Latin)    %Z zone    %z offset\n",
    "    A '-' after the '%' suppresses zero padding, as in %-d.\n",
    "\n",
    "EXIT STATUS\n",
    "    0  success, or --is-holiday and the date is a holiday\n",
    "    1  a date could not be parsed or converted, or --is-holiday and the\n",
    "       date is an ordinary day\n",
    "    2  the command line was wrong\n",
    "\n",
    "EXAMPLES\n",
    "    ethdate                              today, in Amharic\n",
    "    ethdate --locale en -T               today, with the Ethiopian time\n",
    "    ethdate -F gregorian 2024-09-11      -> 2017-01-01\n",
    "    ethdate -g 2017-01-01                -> 2024-09-11\n",
    "    ethdate -f '%A %-d %B %Y'            custom format\n",
    "    seq 0 6 | sed 's/^/+/' | ethdate -   the coming week, from stdin\n",
    "    ethdate --holidays 2018              the year's holidays\n",
    "    ethdate --is-holiday || echo 'work'  script on today's status\n",
    "\n",
    "Report bugs at https://github.com/mitegab/pagumie/issues\n",
);

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(s: &str) -> Options {
        parse(s.split_whitespace().map(String::from).collect()).expect("should parse")
    }

    #[test]
    fn defaults() {
        let o = parse_args("");
        assert_eq!(o.action, Action::Convert);
        assert_eq!(o.from, Calendar::Ethiopic);
        assert_eq!(o.to, Calendar::Ethiopic);
        assert_eq!(o.output, Output::Text);
        assert!(o.inputs.is_empty());
    }

    #[test]
    fn long_options_with_both_spellings() {
        let a = parse_args("--from gregorian --format %Y");
        let b = parse_args("--from=gregorian --format=%Y");
        assert_eq!(a.from, b.from);
        assert_eq!(a.format, b.format);
        assert_eq!(a.format.as_deref(), Some("%Y"));
    }

    #[test]
    fn short_options_bundle() {
        let o = parse_args("-gqu");
        assert_eq!(o.to, Calendar::Gregorian);
        assert!(o.quiet);
        assert_eq!(o.zone, Zone::Utc);
        let o = parse_args("-f%Y-%m");
        assert_eq!(o.format.as_deref(), Some("%Y-%m"));
    }

    #[test]
    fn reading_gregorian_implies_printing_ethiopic() {
        let o = parse_args("-F gregorian 2024-09-11");
        assert_eq!(o.to, Calendar::Ethiopic);
        // ... unless asked otherwise.
        let o = parse_args("-F gregorian --to julian 2024-09-11");
        assert_eq!(o.to, Calendar::Julian);
    }

    #[test]
    fn a_bare_year_after_a_year_action_is_the_year() {
        let o = parse_args("--holidays 2018");
        assert_eq!(o.year, Some(2018));
        assert!(o.inputs.is_empty());
        // But a date stays a date.
        let o = parse_args("--is-holiday 2018-01-01");
        assert_eq!(o.inputs, vec!["2018-01-01".to_string()]);
    }

    #[test]
    fn dash_means_stdin() {
        let o = parse_args("-");
        assert!(o.stdin);
        assert!(o.inputs.is_empty());
    }

    #[test]
    fn double_dash_stops_option_parsing() {
        let o = parse_args("-- --json");
        assert_eq!(o.output, Output::Text);
        assert_eq!(o.inputs, vec!["--json".to_string()]);
    }

    #[test]
    fn negative_relative_days_are_not_options() {
        // '-7' is a bundle of unknown short options as far as a naive parser is
        // concerned; it has to survive as an input.
        let o = parse(vec!["-d".into(), "-7".into()]).unwrap();
        assert_eq!(o.inputs, vec!["-7".to_string()]);
    }

    #[test]
    fn bad_input_is_a_usage_error() {
        assert!(parse(vec!["--nope".into()]).is_err());
        assert!(parse(vec!["--from".into(), "mayan".into()]).is_err());
        assert!(parse(vec!["--format".into()]).is_err());
        assert!(parse(vec!["--offset".into(), "banana".into()]).is_err());
    }

    #[test]
    fn offsets_are_parsed() {
        assert_eq!(parse_args("--offset +03:00").zone, Zone::Fixed(10_800));
        assert_eq!(parse_args("--offset -0500").zone, Zone::Fixed(-18_000));
    }

    #[test]
    fn help_lists_every_long_option_the_parser_accepts() {
        // Keeps the help text honest as options are added.
        for opt in [
            "--date", "--format", "--from", "--to", "--gregorian", "--iso", "--json", "--waybar",
            "--polybar", "--locale", "--era", "--day-start", "--utc", "--offset", "--time",
            "--calendar", "--year", "--holidays", "--computus", "--is-holiday", "--stdin",
            "--null", "--quiet", "--no-color", "--help", "--version",
        ] {
            assert!(HELP.contains(opt), "help text does not mention {opt}");
        }
    }
}
