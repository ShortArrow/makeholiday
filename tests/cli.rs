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

#[test]
fn remove_by_summary_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args(["add", file_str, "--summary", "元日", "--start", "2026-01-01"])
        .assert()
        .success();
    cmd()
        .args(["add", file_str, "--summary", "建国記念の日", "--start", "2026-02-11"])
        .assert()
        .success();

    // remove by summary
    cmd()
        .args(["remove", file_str, "--summary", "元日"])
        .assert()
        .success();

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("建国記念の日"))
        .stdout(predicate::str::contains("元日").not());
}

#[test]
fn remove_by_index_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args(["add", file_str, "--summary", "元日", "--start", "2026-01-01"])
        .assert()
        .success();
    cmd()
        .args(["add", file_str, "--summary", "建国記念の日", "--start", "2026-02-11"])
        .assert()
        .success();

    // remove by index (1-based)
    cmd()
        .args(["remove", file_str, "--index", "2"])
        .assert()
        .success();

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("元日"))
        .stdout(predicate::str::contains("建国記念の日").not());
}

#[test]
fn remove_interactive_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();
    cmd()
        .args(["add", file_str, "--summary", "元日", "--start", "2026-01-01"])
        .assert()
        .success();

    // interactive remove: pipe "1\n" to stdin
    cmd()
        .args(["remove", file_str])
        .write_stdin("1\n")
        .assert()
        .success();

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("元日").not());
}

#[test]
fn add_interactive_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();

    // interactive add: pipe summary, start, end
    cmd()
        .args(["add", file_str])
        .write_stdin("元日\n2026/1/1\n\n")
        .assert()
        .success();

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-01-01 : 元日"));
}

#[test]
fn add_with_summary_and_start_does_not_prompt() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["init", file_str]).assert().success();

    // No stdin provided — should succeed without hanging for end date prompt
    cmd()
        .args(["add", file_str, "--summary", "元日", "--start", "2026-01-01"])
        .assert()
        .success()
        .stderr(predicate::str::contains("End date").not());

    cmd()
        .args(["list", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("1: 2026-01-01 : 元日"));
}
