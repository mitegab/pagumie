//! Turning dates into the various output shapes the CLI supports.

use ethiopic_calendar::bahire_hasab::{BahireHasab, ALL_MOVABLE_FEASTS};
use ethiopic_calendar::clock::EthiopianDateTime;
use ethiopic_calendar::era::Era;
use ethiopic_calendar::format::{format_date, format_datetime, Locale};
use ethiopic_calendar::holidays::{holidays_on, ALL_HOLIDAYS};
use ethiopic_calendar::{EthiopianDate, Result};

use crate::args::{Calendar, Options, Output};

/// An input resolved to a point on the timeline. The time part is present only
/// when the input carried one (`now`, `@timestamp`), because
/// `ethdate 2017-01-01` has no business printing a time of day.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The Ethiopian date.
    pub date: EthiopianDate,
    /// The full local date-time, when the input had one.
    pub datetime: Option<EthiopianDateTime>,
}

/// Default format strings, chosen per locale and per whether a time is shown.
fn default_format(o: &Options, with_time: bool) -> String {
    // `Era` is #[non_exhaustive]; an era we do not know about falls back to
    // the plain Amete Mihret marker rather than failing to build.
    let era = match o.era {
        Era::AmeteAlem => "%L ዓ.ዓ.",
        _ => "%N",
    };
    match (o.locale, with_time) {
        (Locale::Amharic, false) => format!("%A ፣ %-d %B %Y {era}"),
        (Locale::Amharic, true) => format!("%A ፣ %-d %B %Y {era} ፣ %-I:%M %p"),
        (Locale::Latin, false) => "%a %-d %b %Y %E".to_string(),
        (Locale::Latin, true) => "%a %-d %b %Y %E, %-I:%M %P (%H:%M %Z)".to_string(),
    }
}

/// Renders one resolved input according to the options.
pub fn render(r: &Resolved, o: &Options) -> Result<String> {
    match o.output {
        Output::Json => json(r, o),
        Output::Waybar => waybar(r, o),
        Output::Polybar => polybar(r, o),
        Output::Iso => iso(r, o),
        Output::Text => text(r, o),
    }
}

fn iso(r: &Resolved, o: &Options) -> Result<String> {
    Ok(match o.to {
        Calendar::Ethiopic => r.date.to_string(),
        Calendar::Gregorian => r.date.to_gregorian().to_string(),
        Calendar::Julian => r.date.to_julian()?.to_string(),
        Calendar::Jdn => r.date.to_jdn().to_string(),
        Calendar::RataDie => r.date.to_rata_die().get().to_string(),
        Calendar::Unix => match &r.datetime {
            Some(dt) => dt.to_unix_timestamp().to_string(),
            None => (r.date.to_unix_days() * 86_400).to_string(),
        },
    })
}

fn text(r: &Resolved, o: &Options) -> Result<String> {
    // A non-Ethiopian target calendar has no Ethiopian names to print, so the
    // sensible "human" rendering is the ISO one.
    if o.to != Calendar::Ethiopic && o.format.is_none() {
        return iso(r, o);
    }
    let with_time = o.with_time && r.datetime.is_some();
    let fmt = match &o.format {
        Some(f) => f.clone(),
        None => default_format(o, with_time),
    };
    Ok(match &r.datetime {
        Some(dt) => format_datetime(dt, &fmt, o.locale),
        None => format_date(r.date, &fmt, o.locale),
    })
}

/// Escapes a string for inclusion in a JSON document.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn json(r: &Resolved, o: &Options) -> Result<String> {
    let d = r.date;
    let g = d.to_gregorian();
    let holiday = holidays_on(d).next();
    let julian = d
        .to_julian()
        .map_or_else(|_| "null".to_string(), |j| quoted(&j.to_string()));

    let mut s = String::new();
    s.push('{');
    s.push_str(&format!(
        "\"ethiopic\":{{\"year\":{},\"month\":{},\"day\":{},\"month_name\":{},\"month_latin\":{},\"weekday\":{},\"weekday_english\":{},\"day_of_year\":{},\"leap_year\":{},\"amete_alem\":{},\"epagomenal\":{}}}",
        d.year(),
        d.month(),
        d.day(),
        quoted(d.month_enum().amharic()),
        quoted(d.month_enum().latin()),
        quoted(d.weekday().amharic()),
        quoted(d.weekday().english()),
        d.day_of_year(),
        d.is_leap_year(),
        d.amete_alem(),
        d.month() == 13,
    ));
    s.push_str(&format!(",\"gregorian\":{}", quoted(&g.to_string())));
    s.push_str(&format!(",\"julian\":{julian}"));
    s.push_str(&format!(",\"jdn\":{}", d.to_jdn()));
    s.push_str(&format!(",\"rata_die\":{}", d.to_rata_die().get()));
    s.push_str(&format!(",\"unix\":{}", d.to_unix_days() * 86_400));
    s.push_str(&format!(
        ",\"formatted\":{}",
        quoted(&text(
            r,
            &Options {
                output: Output::Text,
                to: Calendar::Ethiopic,
                ..o.clone()
            }
        )?)
    ));
    match holiday {
        Some(h) => s.push_str(&format!(
            ",\"holiday\":{{\"key\":{},\"amharic\":{},\"english\":{},\"public\":{}}}",
            quoted(h.key()),
            quoted(h.amharic()),
            quoted(h.english()),
            h.is_public_holiday()
        )),
        None => s.push_str(",\"holiday\":null"),
    }
    if let Some(dt) = &r.datetime {
        let t = dt.time();
        s.push_str(&format!(
            ",\"time\":{{\"ethiopian_hour\":{},\"minute\":{},\"second\":{},\"wall_hour\":{},\"period\":{},\"period_english\":{},\"utc_offset\":{},\"unix\":{}}}",
            t.ethiopian_hour(),
            t.minute(),
            t.second(),
            t.wall_hour(),
            quoted(t.period().amharic()),
            quoted(t.period().english()),
            dt.utc_offset_seconds(),
            dt.to_unix_timestamp(),
        ));
    }
    s.push('}');
    Ok(s)
}

/// Waybar's `custom` module reads a JSON object with `text`, `alt`, `tooltip`
/// and `class` keys; `class` drives the CSS, so holidays can be styled.
fn waybar(r: &Resolved, o: &Options) -> Result<String> {
    let d = r.date;
    let text_fmt = o.format.clone().unwrap_or_else(|| match o.locale {
        Locale::Amharic => "%-d %B %Y".to_string(),
        Locale::Latin => "%-d %b %Y".to_string(),
    });
    let text = match &r.datetime {
        Some(dt) => format_datetime(dt, &text_fmt, o.locale),
        None => format_date(d, &text_fmt, o.locale),
    };
    let holiday = holidays_on(d).next();
    let class = match holiday {
        Some(h) if h.is_public_holiday() => "holiday",
        Some(_) => "observance",
        None if d.month() == 13 => "pagume",
        None => "ordinary",
    };

    let mut tip = String::new();
    tip.push_str(&format_date(d, "%A ፣ %-d %B %Y ዓ.ም.", Locale::Amharic));
    tip.push('\n');
    tip.push_str(&format_date(d, "%A, %-d %B %Y E.C.", Locale::Latin));
    tip.push('\n');
    tip.push_str(&format!("Gregorian: {}\n", d.to_gregorian()));
    tip.push_str(&format!(
        "Day {} of {}, {}\n",
        d.day_of_year(),
        d.days_in_year(),
        if d.is_leap_year() {
            "leap year (Pagume has 6 days)"
        } else {
            "ordinary year"
        }
    ));
    if let Some(dt) = &r.datetime {
        tip.push_str(&format!(
            "Ethiopian time: {}\n",
            format_datetime(dt, "%-I:%M %p (%H:%M %Z)", o.locale)
        ));
    }
    if let Some(h) = holiday {
        tip.push_str(&format!("\n★ {} — {}\n", h.amharic(), h.english()));
    }
    if let Some(next) = next_holiday(d) {
        let (h, date) = next;
        let days = date.days_since(d);
        if days > 0 {
            tip.push_str(&format!(
                "\nNext: {} ({}) in {} day{}\n",
                h.english(),
                date,
                days,
                if days == 1 { "" } else { "s" }
            ));
        }
    }

    Ok(format!(
        "{{\"text\":{},\"alt\":{},\"tooltip\":{},\"class\":{}}}",
        quoted(&text),
        quoted(class),
        quoted(tip.trim_end()),
        quoted(class)
    ))
}

fn polybar(r: &Resolved, o: &Options) -> Result<String> {
    let d = r.date;
    let fmt = o.format.clone().unwrap_or_else(|| match o.locale {
        Locale::Amharic => "%-d %B %Y".to_string(),
        Locale::Latin => "%-d %b %Y".to_string(),
    });
    let base = match &r.datetime {
        Some(dt) => format_datetime(dt, &fmt, o.locale),
        None => format_date(d, &fmt, o.locale),
    };
    Ok(match holidays_on(d).next() {
        Some(h) if h.is_public_holiday() => format!("{base} ★ {}", h.english()),
        _ => base,
    })
}

/// The next holiday strictly after `date`, searching this year and the next.
pub fn next_holiday(date: EthiopianDate) -> Option<(ethiopic_calendar::Holiday, EthiopianDate)> {
    let mut best: Option<(ethiopic_calendar::Holiday, EthiopianDate)> = None;
    for year in [date.year(), date.year() + 1] {
        for h in ALL_HOLIDAYS {
            let Ok(d) = h.observed_date(year) else {
                continue;
            };
            let improves = match best {
                None => true,
                Some((_, current)) => d < current,
            };
            if d > date && improves {
                best = Some((h, d));
            }
        }
    }
    best
}

/// The `--holidays` listing.
pub fn holidays_table(year: i32, o: &Options) -> Result<String> {
    let mut rows: Vec<(EthiopianDate, String, String, bool)> = Vec::new();
    for h in ALL_HOLIDAYS {
        let d = h.observed_date(year)?;
        rows.push((
            d,
            h.amharic().to_string(),
            h.english().to_string(),
            h.is_public_holiday(),
        ));
    }
    rows.sort_by_key(|r| r.0);

    if o.output == Output::Json {
        let items: Vec<String> = rows
            .iter()
            .map(|(d, am, en, public)| {
                format!(
                    "{{\"ethiopic\":{},\"gregorian\":{},\"weekday\":{},\"amharic\":{},\"english\":{},\"public\":{}}}",
                    quoted(&d.to_string()),
                    quoted(&d.to_gregorian().to_string()),
                    quoted(d.weekday().english()),
                    quoted(am),
                    quoted(en),
                    public
                )
            })
            .collect();
        return Ok(format!(
            "{{\"year\":{year},\"holidays\":[{}]}}",
            items.join(",")
        ));
    }

    let mut out = String::new();
    let name_width = rows
        .iter()
        .map(|(_, am, en, _)| {
            if o.locale == Locale::Amharic { am } else { en }
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    for (d, am, en, public) in rows {
        let name = if o.locale == Locale::Amharic { am } else { en };
        let pad = name_width.saturating_sub(name.chars().count());
        out.push_str(&format!(
            "{}  {}  {name}{:pad$}  {}\n",
            d,
            d.to_gregorian(),
            "",
            if public {
                "public holiday"
            } else {
                "observance"
            },
            pad = pad
        ));
    }
    Ok(out.trim_end().to_string())
}

/// The `--computus` table.
pub fn computus_table(year: i32, o: &Options) -> Result<String> {
    let bh = BahireHasab::new(year)?;
    if o.output == Output::Json {
        let feasts: Vec<String> = ALL_MOVABLE_FEASTS
            .iter()
            .filter_map(|f| bh.feast(*f).ok().map(|d| (f, d)))
            .map(|(f, d)| {
                format!(
                    "{{\"key\":{},\"english\":{},\"amharic\":{},\"ethiopic\":{},\"gregorian\":{}}}",
                    quoted(f.key()),
                    quoted(f.english()),
                    quoted(f.amharic()),
                    quoted(&d.to_string()),
                    quoted(&d.to_gregorian().to_string())
                )
            })
            .collect();
        return Ok(format!(
            "{{\"year\":{year},\"amete_alem\":{},\"medeb\":{},\"wenber\":{},\"abektie\":{},\"metqi\":{},\"beale_metqi\":{},\"mebaja_hamer\":{},\"tewsak\":{},\"evangelist\":{},\"new_year_weekday\":{},\"feasts\":[{}]}}",
            bh.amete_alem(),
            bh.medeb(),
            bh.wenber(),
            bh.abektie(),
            bh.metqi(),
            quoted(&bh.beale_metqi().to_string()),
            bh.mebaja_hamer(),
            bh.tewsak(),
            quoted(bh.evangelist().english()),
            quoted(bh.new_year_weekday().english()),
            feasts.join(",")
        ));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "ባሕረ ሐሳብ — Baḥire Ḥasab for {year} E.C. (Amete Alem {})\n\n",
        bh.amete_alem()
    ));
    out.push_str(&format!(
        "  {:<26}{}\n",
        "ወንጌላዊ / Evangelist",
        match o.locale {
            Locale::Amharic => bh.evangelist().amharic(),
            Locale::Latin => bh.evangelist().english(),
        }
    ));
    out.push_str(&format!(
        "  {:<26}{}\n",
        "መባጃ ሐመር / new year",
        bh.new_year_weekday().english()
    ));
    out.push_str(&format!("  {:<26}{}\n", "መደብ / medeb", bh.medeb()));
    out.push_str(&format!("  {:<26}{}\n", "ወንበር / wenber", bh.wenber()));
    out.push_str(&format!("  {:<26}{}\n", "አበቅቴ / abektie", bh.abektie()));
    out.push_str(&format!("  {:<26}{}\n", "መጥቅዕ / metqi", bh.metqi()));
    out.push_str(&format!(
        "  {:<26}{}\n",
        "በዓለ መጥቅዕ / beale metqi",
        bh.beale_metqi()
    ));
    out.push_str(&format!("  {:<26}{}\n", "ጠውሳክ / tewsak", bh.tewsak()));
    out.push_str(&format!(
        "  {:<26}{}\n\n",
        "መባጃ ሐመር / mebaja hamer",
        bh.mebaja_hamer()
    ));

    let width = ALL_MOVABLE_FEASTS
        .iter()
        .map(|f| f.english().chars().count())
        .max()
        .unwrap_or(0);
    for f in ALL_MOVABLE_FEASTS {
        let d = bh.feast(f)?;
        out.push_str(&format!(
            "  {:<width$}  {}  {}  {}\n",
            f.english(),
            d,
            d.to_gregorian(),
            f.amharic(),
            width = width
        ));
    }
    Ok(out.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options {
            locale: Locale::Latin,
            ..Options::default()
        }
    }

    fn resolved(y: i32, m: u8, d: u8) -> Resolved {
        Resolved {
            date: EthiopianDate::from_ymd(y, m, d).unwrap(),
            datetime: None,
        }
    }

    #[test]
    fn json_is_well_formed_and_escaped() {
        let mut o = opts();
        o.output = Output::Json;
        let s = json(&resolved(2017, 1, 1), &o).unwrap();
        assert!(s.starts_with('{') && s.ends_with('}'));
        assert!(s.contains("\"gregorian\":\"2024-09-11\""));
        assert!(s.contains("\"holiday\":{\"key\":\"enkutatash\""));
        assert!(s.contains("\"jdn\":"));
        // Balanced braces and quotes: a crude but effective well-formedness check.
        assert_eq!(
            s.chars().filter(|&c| c == '{').count(),
            s.chars().filter(|&c| c == '}').count()
        );
        assert_eq!(s.matches('"').count() % 2, 0);
    }

    #[test]
    fn json_escapes_control_characters() {
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
        assert_eq!(json_escape("\u{1}"), "\\u0001");
    }

    #[test]
    fn iso_output_per_calendar() {
        let r = resolved(2017, 1, 1);
        let mut o = opts();
        o.output = Output::Iso;
        for (cal, want) in [
            (Calendar::Ethiopic, "2017-01-01"),
            (Calendar::Gregorian, "2024-09-11"),
            (Calendar::Julian, "2024-08-29"),
        ] {
            o.to = cal;
            assert_eq!(iso(&r, &o).unwrap(), want, "calendar {cal:?}");
        }
        o.to = Calendar::Jdn;
        assert_eq!(iso(&r, &o).unwrap(), "2460565");
    }

    #[test]
    fn waybar_marks_holidays_with_a_css_class() {
        let mut o = opts();
        o.output = Output::Waybar;
        let s = waybar(&resolved(2017, 1, 1), &o).unwrap();
        assert!(s.contains("\"class\":\"holiday\""), "{s}");
        assert!(s.contains("\\n"), "tooltip newlines must be escaped: {s}");
        let s = waybar(&resolved(2017, 2, 3), &o).unwrap();
        assert!(s.contains("\"class\":\"ordinary\""), "{s}");
        let s = waybar(&resolved(2015, 13, 6), &o).unwrap();
        assert!(s.contains("\"class\":\"pagume\""), "{s}");
    }

    #[test]
    fn next_holiday_moves_forward_across_the_year_boundary() {
        // Ṗagumēn is after the last holiday of the year, so the next one is
        // Enkutatash in the following year.
        let d = EthiopianDate::from_ymd(2015, 13, 5).unwrap();
        let (h, date) = next_holiday(d).unwrap();
        assert_eq!(h.key(), "enkutatash");
        assert_eq!(date.year(), 2016);
        assert!(date > d);
    }

    #[test]
    fn holidays_table_is_sorted_and_complete() {
        let o = opts();
        let t = holidays_table(2017, &o).unwrap();
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines.len(), ALL_HOLIDAYS.len());
        let dates: Vec<&str> = lines
            .iter()
            .map(|l| l.split_whitespace().next().unwrap())
            .collect();
        let mut sorted = dates.clone();
        sorted.sort_unstable();
        assert_eq!(dates, sorted, "holidays must be listed in date order");
    }

    #[test]
    fn computus_table_mentions_the_parameters() {
        let o = opts();
        let t = computus_table(2017, &o).unwrap();
        for needle in ["medeb", "wenber", "abektie", "metqi", "Easter", "7517"] {
            assert!(t.contains(needle), "computus table lacks {needle}:\n{t}");
        }
    }
}
