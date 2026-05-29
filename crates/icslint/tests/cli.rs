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
