use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("makeholiday").unwrap()
}

#[test]
fn init_add_list_workflow() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    // init
    cmd()
        .args(["init", file_str])
        .assert()
        .success();

    // add single day
    cmd()
        .args(["add", file_str, "--summary", "元日", "--start", "2026-01-01"])
        .assert()
        .success();

    // add multi day
    cmd()
        .args([
            "add", file_str, "--summary", "年末年始", "--start", "2026-12-29", "--end", "2027-01-03",
        ])
        .assert()
        .success();

    // list
    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-01-01 : 元日"))
        .stdout(predicate::str::contains("2026-12-29 to 2027-01-03 : 年末年始"));
}

#[test]
fn init_default_file() {
    let dir = TempDir::new().unwrap();

    // init with default file
    cmd()
        .current_dir(dir.path())
        .args(["init"])
        .assert()
        .success();

    assert!(dir.path().join("calendar.ics").exists());

    // add with default file
    cmd()
        .current_dir(dir.path())
        .args(["add", "--summary", "テスト", "--start", "2026-06-01"])
        .assert()
        .success();

    // list with default file
    cmd()
        .current_dir(dir.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-06-01 : テスト"));
}

#[test]
fn init_fails_on_existing_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args(["init", file_str])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn add_end_before_start_fails() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args([
            "add", file_str, "--summary", "invalid", "--start", "2026-03-01", "--end", "2026-02-01",
        ])
        .assert()
        .failure();
}

#[test]
fn add_with_slash_date_format() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args(["add", file_str, "--summary", "夏季休業", "--start", "2026/4/8", "--end", "2026/5/3"])
        .assert()
        .success();

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-04-08 to 2026-05-03 : 夏季休業"));
}
