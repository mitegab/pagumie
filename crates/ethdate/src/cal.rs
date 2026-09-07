//! `cal(1)` for the Ethiopian calendar.
//!
//! Ṗagumēn is the interesting part of the layout: a five- or six-day thirteenth
//! month that must be shown as a real month rather than folded into Näḥase,
//! which is what most calendar widgets do wrong.

use ethiopic_calendar::format::Locale;
use ethiopic_calendar::holidays::is_holiday;
use ethiopic_calendar::weekday::ALL_WEEKDAYS;
use ethiopic_calendar::{ethiopic, EthiopianDate, Result};

/// How wide one month block is: seven columns of two digits, space separated.
const BLOCK_WIDTH: usize = 20;

/// Styling for the grid.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Script for the month and weekday headings.
    pub locale: Locale,
    /// Emit ANSI escapes to highlight today and holidays.
    pub color: bool,
    /// The date to highlight, if any.
    pub today: Option<EthiopianDate>,
}

fn weekday_heading(locale: Locale) -> String {
    let mut s = String::new();
    for w in ALL_WEEKDAYS {
        let cell = match locale {
            // The Amharic names all begin with a distinct character, which is
            // exactly what a two-column heading needs.
            Locale::Amharic => w
                .amharic()
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default(),
            Locale::Latin => w.english_abbrev().chars().take(2).collect(),
        };
        s.push_str(&format!("{cell:>2} "));
    }
    s.trim_end().to_string()
}

/// Renders one month as a block of lines, each padded to [`BLOCK_WIDTH`].
///
/// Padding is by *character* count, not byte count, so Amharic headings line up
/// in a terminal.
pub fn month_block(year: i32, month: u8, style: Style) -> Result<Vec<String>> {
    let first = EthiopianDate::from_ymd(year, month, 1)?;
    let days = ethiopic::days_in_month(year, month);
    let title = match style.locale {
        Locale::Amharic => format!("{} {year}", first.month_enum().amharic()),
        Locale::Latin => format!("{} {year}", first.month_enum().latin()),
    };

    let mut lines = vec![center(&title, BLOCK_WIDTH), weekday_heading(style.locale)];
    let lead = first.weekday().number() as usize;
    let mut cells: Vec<String> = vec!["  ".to_string(); lead];
    for day in 1..=days {
        let date = EthiopianDate::from_ymd(year, month, day)?;
        let mut cell = format!("{day:>2}");
        if style.color {
            if style.today == Some(date) {
                cell = format!("\u{1b}[7m{cell}\u{1b}[0m");
            } else if is_holiday(date) {
                cell = format!("\u{1b}[1;31m{cell}\u{1b}[0m");
            }
        }
        cells.push(cell);
    }
    for week in cells.chunks(7) {
        let mut line = week.join(" ");
        // Trailing blanks are trimmed, then the block is re-padded, so that a
        // month block never has ragged whitespace when joined side by side.
        line = line.trim_end().to_string();
        lines.push(line);
    }
    while lines.len() < 8 {
        lines.push(String::new());
    }
    Ok(lines.into_iter().map(|l| pad_to(&l, BLOCK_WIDTH)).collect())
}

/// A single month, as printable text.
pub fn month(year: i32, month_number: u8, style: Style) -> Result<String> {
    Ok(month_block(year, month_number, style)?
        .into_iter()
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string())
}

/// A whole Ethiopian year: thirteen months, three to a row, so Ṗagumēn sits
/// beside Näḥase instead of being hidden.
pub fn year(year_number: i32, style: Style) -> Result<String> {
    let blocks: Vec<Vec<String>> = (1..=13)
        .map(|m| month_block(year_number, m, style))
        .collect::<Result<_>>()?;
    let mut out = String::new();
    out.push_str(&center(&format!("{year_number} ዓ.ም."), BLOCK_WIDTH * 3 + 4));
    out.push_str("\n\n");
    for row in blocks.chunks(3) {
        let height = row.iter().map(Vec::len).max().unwrap_or(0);
        for i in 0..height {
            let line: Vec<String> = row
                .iter()
                .map(|b| b.get(i).cloned().unwrap_or_else(|| " ".repeat(BLOCK_WIDTH)))
                .collect();
            out.push_str(line.join("  ").trim_end());
            out.push('\n');
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn char_len(s: &str) -> usize {
    // ANSI escapes are zero-width; strip them before measuring.
    let mut n = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        if c == '\u{1b}' {
            in_escape = true;
            continue;
        }
        n += 1;
    }
    n
}

fn pad_to(s: &str, width: usize) -> String {
    let len = char_len(s);
    if len >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}

fn center(s: &str, width: usize) -> String {
    let len = char_len(s);
    if len >= width {
        return s.to_string();
    }
    let left = (width - len) / 2;
    format!("{}{s}", " ".repeat(left))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Style {
        Style {
            locale: Locale::Latin,
            color: false,
            today: None,
        }
    }

    #[test]
    fn a_month_has_a_title_a_heading_and_the_right_days() {
        let block = month_block(2017, 1, plain()).unwrap();
        assert!(block[0].contains("Meskerem 2017"));
        assert!(block[1].starts_with("Su"), "{:?}", block[1]);
        let text = block.join("\n");
        assert!(text.contains("30"));
        assert!(!text.contains("31"), "Ethiopian months have thirty days");
    }

    #[test]
    fn pagume_is_a_real_month_with_five_or_six_days() {
        let ordinary = month(2016, 13, plain()).unwrap();
        assert!(ordinary.contains(" 5"));
        assert!(
            !ordinary.contains(" 6"),
            "2016 is not a leap year:\n{ordinary}"
        );
        let leap = month(2015, 13, plain()).unwrap();
        assert!(leap.contains(" 6"), "2015 is a leap year:\n{leap}");
    }

    #[test]
    fn the_first_day_lands_under_its_weekday() {
        // 1 Meskerem 2017 is a Wednesday (11 September 2024).
        let first = EthiopianDate::from_ymd(2017, 1, 1).unwrap();
        assert_eq!(first.weekday().english(), "Wednesday");
        let block = month_block(2017, 1, plain()).unwrap();
        let week = &block[2];
        // Three weekday columns of three characters precede Wednesday's.
        assert!(week.starts_with("          1"), "{week:?}");
    }

    #[test]
    fn every_block_is_rectangular() {
        for m in 1..=13 {
            let block = month_block(2015, m, plain()).unwrap();
            assert_eq!(block.len(), 8, "month {m} should be eight lines tall");
            for line in &block {
                assert_eq!(char_len(line), BLOCK_WIDTH, "month {m}: {line:?}");
            }
        }
    }

    #[test]
    fn the_year_view_shows_all_thirteen_months() {
        let text = year(2017, plain()).unwrap();
        for m in ethiopic::ALL_MONTHS {
            assert!(
                text.contains(m.latin()),
                "year view is missing {}",
                m.latin()
            );
        }
    }

    #[test]
    fn colour_is_only_emitted_when_asked_for() {
        let style = Style {
            color: true,
            today: Some(EthiopianDate::from_ymd(2017, 1, 2).unwrap()),
            ..plain()
        };
        let coloured = month(2017, 1, style).unwrap();
        assert!(coloured.contains('\u{1b}'));
        assert!(!month(2017, 1, plain()).unwrap().contains('\u{1b}'));
        // Escapes must not disturb the layout.
        for line in month_block(2017, 1, style).unwrap() {
            assert_eq!(char_len(&line), BLOCK_WIDTH);
        }
    }
}
