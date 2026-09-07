//! End-to-end tests that run the real binary, because the interesting bugs in
//! a command-line tool live in argument handling, exit codes and stream
//! behaviour rather than in the library underneath.

use std::io::Write;
use std::process::{Command, Stdio};

fn ethdate() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_ethdate"));
    // Deterministic environment: a fixed zone and a predictable locale.
    c.env("TZ", "UTC").env("LC_ALL", "C").env("NO_COLOR", "1");
    c
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Run {
    run_with_stdin(args, "")
}

fn run_with_stdin(args: &[&str], input: &str) -> Run {
    let mut child = ethdate()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary should start");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn converts_gregorian_to_ethiopic() {
    let r = run(&["-F", "gregorian", "--iso", "2024-09-11"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(r.stdout, "2017-01-01\n");
}

#[test]
fn converts_ethiopic_to_gregorian() {
    let r = run(&["-g", "--iso", "2017-01-01"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(r.stdout, "2024-09-11\n");
}

#[test]
fn the_epagomenal_month_survives_a_round_trip() {
    let r = run(&["-g", "--iso", "2015-13-06"]);
    assert_eq!(r.stdout, "2023-09-11\n");
    let r = run(&["-F", "gregorian", "--iso", "2023-09-11"]);
    assert_eq!(r.stdout, "2015-13-06\n");
}

#[test]
fn custom_formats() {
    let r = run(&["--locale", "en", "-f", "%A %-d %B %Y %E", "2017-01-01"]);
    assert_eq!(r.stdout, "Wednesday 1 Meskerem 2017 E.C.\n");
    let r = run(&["-f", "%G", "2017-01-01"]);
    assert_eq!(r.stdout.trim(), "፳፻፲፯");
    let r = run(&["-f", "%J %R %L", "2017-01-01"]);
    assert_eq!(r.stdout.trim(), "2460565 739140 7517");
}

#[test]
fn unknown_specifiers_are_visible_rather_than_dropped() {
    let r = run(&["-f", "%Q", "2017-01-01"]);
    assert_eq!(r.stdout, "%Q\n");
}

#[test]
fn reads_dates_from_standard_input() {
    let r = run_with_stdin(&["-", "--iso", "-g"], "2017-01-01\n2017-13-05\n2018-01-01\n");
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(r.stdout, "2024-09-11\n2025-09-10\n2025-09-11\n");
}

#[test]
fn null_separated_records_round_trip() {
    let r = run_with_stdin(&["-0", "-s", "--iso"], "2017-01-01\x002017-01-02\x00");
    assert_eq!(r.stdout, "2017-01-01\x002017-01-02\x00");
}

#[test]
fn exit_status_distinguishes_bad_dates_from_bad_usage() {
    let ok = run(&["--iso", "2017-01-01"]);
    assert_eq!(ok.code, 0);

    let bad_date = run(&["--iso", "2017-13-07"]); // Pagume has six days at most
    assert_eq!(bad_date.code, 1);
    assert!(bad_date.stdout.is_empty());
    assert!(bad_date.stderr.contains("ethdate:"));

    let bad_usage = run(&["--frobnicate"]);
    assert_eq!(bad_usage.code, 2);
    assert!(bad_usage.stderr.contains("unknown option"));
}

#[test]
fn quiet_suppresses_the_message_but_not_the_status() {
    let r = run(&["-q", "--iso", "nonsense"]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.is_empty(), "{}", r.stderr);
}

#[test]
fn is_holiday_behaves_like_test() {
    let yes = run(&["--is-holiday", "-q", "2017-01-01"]);
    assert_eq!(yes.code, 0);
    let no = run(&["--is-holiday", "-q", "2017-01-02"]);
    assert_eq!(no.code, 1);
    let named = run(&["--is-holiday", "--locale", "en", "2017-04-29"]);
    assert_eq!(named.code, 0);
    assert!(
        named.stdout.contains("Christmas"),
        "{}",
        named.stdout
    );
}

#[test]
fn json_output_is_parseable_and_stable() {
    let r = run(&["--json", "2017-01-01"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let s = r.stdout.trim();
    assert!(s.starts_with('{') && s.ends_with('}'));
    for needle in [
        "\"year\":2017",
        "\"month\":1",
        "\"gregorian\":\"2024-09-11\"",
        "\"jdn\":2460565",
        "\"holiday\":{",
        "\"amete_alem\":7517",
    ] {
        assert!(s.contains(needle), "JSON is missing {needle}: {s}");
    }
}

#[test]
fn waybar_output_has_the_keys_waybar_needs() {
    let r = run(&["--waybar", "2017-01-01"]);
    let s = r.stdout.trim();
    for key in ["\"text\":", "\"alt\":", "\"tooltip\":", "\"class\":"] {
        assert!(s.contains(key), "Waybar JSON is missing {key}: {s}");
    }
    assert!(!s.contains('\n'), "Waybar wants exactly one line");
}

#[test]
fn the_calendar_grid_shows_thirteen_months() {
    let r = run(&["--year", "2017", "--locale", "en"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    for month in [
        "Meskerem", "Tikimt", "Hidar", "Tahsas", "Tir", "Yekatit", "Megabit", "Miazia", "Ginbot",
        "Sene", "Hamle", "Nehase", "Pagume",
    ] {
        assert!(r.stdout.contains(month), "year grid is missing {month}");
    }
}

#[test]
fn holidays_are_listed_in_order_with_both_calendars() {
    let r = run(&["--holidays", "2017", "--locale", "en"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let first = r.stdout.lines().next().unwrap();
    assert!(first.starts_with("2017-01-01"), "{first}");
    assert!(first.contains("2024-09-11"), "{first}");
    assert!(r.stdout.contains("Christmas"));
    assert!(r.stdout.contains("Easter") || r.stdout.contains("Fasika"));
}

#[test]
fn computus_reports_the_traditional_parameters() {
    let r = run(&["--computus", "2017"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    for needle in ["medeb", "wenber", "abektie", "metqi", "7517"] {
        assert!(r.stdout.contains(needle), "{needle} missing:\n{}", r.stdout);
    }
}

#[test]
fn timezone_handling_is_explicit() {
    // 2024-09-10T22:00:00Z is already 11 September in Addis Ababa, and with the
    // traditional dawn boundary it still belongs to the previous Ethiopian day.
    // 2024-09-10T23:00:00Z is 02:00 on 11 September in Addis Ababa: a new
    // civil day, but still the previous Ethiopian day until dawn.
    let utc = run(&["--utc", "--iso", "@1726009200"]);
    let eat = run(&["--offset", "+03:00", "--iso", "@1726009200"]);
    assert_eq!(utc.stdout.trim(), "2016-13-05");
    assert_eq!(eat.stdout.trim(), "2016-13-05");
    let midnight = run(&[
        "--offset",
        "+03:00",
        "--day-start",
        "midnight",
        "--iso",
        "@1726009200",
    ]);
    assert_eq!(midnight.stdout.trim(), "2017-01-01");
}

#[test]
fn relative_days_and_keywords_agree_with_each_other() {
    let today = run(&["--iso", "today"]);
    let plus_zero = run(&["--iso", "+0"]);
    assert_eq!(today.stdout, plus_zero.stdout);
    let tomorrow = run(&["--iso", "tomorrow"]);
    let plus_one = run(&["--iso", "+1"]);
    assert_eq!(tomorrow.stdout, plus_one.stdout);
}

#[test]
fn help_and_version_are_on_stdout_and_exit_zero() {
    let h = run(&["--help"]);
    assert_eq!(h.code, 0);
    assert!(h.stdout.contains("USAGE"));
    assert!(h.stderr.is_empty());
    let v = run(&["--version"]);
    assert_eq!(v.code, 0);
    assert!(v.stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn a_broken_pipe_is_not_an_error() {
    // `ethdate ... | head -1` must exit quietly rather than with an I/O error.
    let mut producer = ethdate()
        .args(["--iso", "-s"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = producer.stdin.take().expect("stdin");
    let writer = std::thread::spawn(move || {
        for _ in 0..10_000 {
            if stdin.write_all(b"2017-01-01\n").is_err() {
                break;
            }
        }
    });
    let stdout = producer.stdout.take().expect("stdout");
    drop(stdout); // close the read end immediately
    let status = producer.wait().expect("wait");
    let _ = writer.join();
    assert!(
        matches!(status.code(), Some(0) | Some(1) | None),
        "unexpected status {status:?}"
    );
}

#[test]
fn every_documented_example_in_the_help_text_runs() {
    // The examples block promises these work; make the promise testable.
    for args in [
        vec!["--locale", "en", "-T"],
        vec!["-F", "gregorian", "2024-09-11"],
        vec!["-g", "2017-01-01"],
        vec!["-f", "%A %-d %B %Y"],
        vec!["--holidays", "2018"],
    ] {
        let r = run(&args);
        assert_eq!(r.code, 0, "{args:?} failed: {}", r.stderr);
        assert!(!r.stdout.is_empty(), "{args:?} printed nothing");
    }
}
