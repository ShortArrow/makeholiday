//! `icslint` CLI integration tests.
//!
//! Exercises the binary entrypoint (clap parse → lint → human reporter →
//! exit code) end-to-end against fixture inputs written into a tempdir.
//! ADR-026 §"Exit codes" is the source of truth for the 0 / 1 / 2 / 3
//! codes asserted below.

use std::fs;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn write_fixture(name: &str, content: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join(name);
    fs::write(&path, content).expect("write fixture");
    (tmp, path)
}

const CLEAN_VEVENT: &str = "\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//x//y\r\n\
BEGIN:VEVENT\r\n\
UID:event-1\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:Showa Day\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

const MISSING_UID: &str = "\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//x//y\r\n\
BEGIN:VEVENT\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260429\r\n\
DTEND;VALUE=DATE:20260430\r\n\
SUMMARY:Showa Day\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

#[test]
fn clean_calendar_exits_zero_with_no_output() {
    let (_tmp, path) = write_fixture("clean.ics", CLEAN_VEVENT);
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&path)
        .assert()
        .success()
        .stderr("");
}

#[test]
fn missing_uid_exits_two_with_diagnostic() {
    let (_tmp, path) = write_fixture("missing-uid.ics", MISSING_UID);
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&path)
        .assert()
        .code(2)
        .stderr(contains("RFC5545/required-uid"));
}

#[test]
fn missing_uid_diagnostic_includes_filename_and_severity_label() {
    let (_tmp, path) = write_fixture("missing-uid.ics", MISSING_UID);
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&path)
        .assert()
        .code(2)
        .stderr(contains("error"))
        .stderr(contains("missing-uid.ics"));
}

#[test]
fn unreadable_file_exits_three() {
    let tmp = TempDir::new().unwrap();
    let bogus = tmp.path().join("does-not-exist.ics");
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&bogus)
        .assert()
        .code(3)
        .stderr(contains("cannot read"));
}

#[test]
fn unparseable_input_exits_two_with_parse_error_diagnostic() {
    let (_tmp, path) = write_fixture("garbage.ics", "not an ics file at all\n");
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&path)
        .assert()
        .code(2)
        .stderr(contains("RFC5545/parse-error"));
}

#[test]
fn stdin_dash_reads_from_stdin() {
    Command::cargo_bin("icslint")
        .unwrap()
        .arg("-")
        .write_stdin(CLEAN_VEVENT)
        .assert()
        .success();
}

#[test]
fn multiple_files_each_get_diagnostics() {
    let (_a, clean) = write_fixture("a.ics", CLEAN_VEVENT);
    let (_b, missing) = write_fixture("b.ics", MISSING_UID);
    Command::cargo_bin("icslint")
        .unwrap()
        .args([&clean, &missing])
        .assert()
        .code(2)
        .stderr(contains("RFC5545/required-uid"));
}

#[test]
fn json_format_emits_array_on_stdout() {
    let (_tmp, path) = write_fixture("missing-uid.ics", MISSING_UID);
    let out = Command::cargo_bin("icslint")
        .unwrap()
        .args([path.to_str().unwrap(), "-f", "json"])
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let s = std::str::from_utf8(&out).expect("utf-8");
    // Top-level must parse as a JSON array containing the rule id.
    let parsed: serde_json::Value = serde_json::from_str(s).expect("valid json");
    let arr = parsed.as_array().expect("array");
    assert!(arr.iter().any(|row| row["rule"] == "RFC5545/required-uid"));
}

#[test]
fn json_format_emits_empty_array_for_clean_input() {
    let (_tmp, path) = write_fixture("clean.ics", CLEAN_VEVENT);
    let out = Command::cargo_bin("icslint")
        .unwrap()
        .args([path.to_str().unwrap(), "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = std::str::from_utf8(&out).expect("utf-8");
    let parsed: serde_json::Value = serde_json::from_str(s).expect("valid json");
    assert_eq!(parsed.as_array().map(|a| a.len()), Some(0));
}

#[test]
fn github_format_emits_workflow_commands_on_stdout() {
    let (_tmp, path) = write_fixture("missing-uid.ics", MISSING_UID);
    Command::cargo_bin("icslint")
        .unwrap()
        .args([path.to_str().unwrap(), "-f", "github"])
        .assert()
        .code(2)
        .stdout(contains("::error file="))
        .stdout(contains("title=RFC5545/required-uid"));
}

#[test]
fn human_format_diagnostics_do_not_leak_to_stdout() {
    // Regression guard: machine-format consumers rely on the human
    // reporter staying on stderr; if a refactor accidentally swaps
    // streams this test catches it.
    let (_tmp, path) = write_fixture("missing-uid.ics", MISSING_UID);
    Command::cargo_bin("icslint")
        .unwrap()
        .arg(&path)
        .assert()
        .code(2)
        .stdout(predicates::str::is_empty());
}
