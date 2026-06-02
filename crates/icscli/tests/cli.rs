use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("icscli").unwrap()
}

#[test]
fn init_add_list_workflow() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    // init
    cmd().args(["-f", file_str, "init"]).assert().success();

    // add single day
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();

    // add multi day
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "年末年始",
            "--start",
            "2026-12-29",
            "--end",
            "2027-01-03",
        ])
        .assert()
        .success();

    // list
    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-01-01 : 元日"))
        .stdout(predicate::str::contains(
            "2026-12-29 to 2027-01-03 : 年末年始",
        ));
}

#[test]
fn init_default_file() {
    let dir = TempDir::new().unwrap();

    cmd()
        .current_dir(dir.path())
        .args(["init"])
        .assert()
        .success();

    assert!(dir.path().join("calendar.ics").exists());

    cmd()
        .current_dir(dir.path())
        .args(["add", "--summary", "テスト", "--start", "2026-06-01"])
        .assert()
        .success();

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

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args(["-f", file_str, "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn add_end_before_start_fails() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "invalid",
            "--start",
            "2026-03-01",
            "--end",
            "2026-02-01",
        ])
        .assert()
        .failure();
}

#[test]
fn add_with_slash_date_format() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "夏季休業",
            "--start",
            "2026/4/8",
            "--end",
            "2026/5/3",
        ])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "2026-04-08 to 2026-05-03 : 夏季休業",
        ));
}

#[test]
fn remove_by_summary_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "建国記念の日",
            "--start",
            "2026-02-11",
        ])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "remove", "--summary", "元日"])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "list"])
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

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "建国記念の日",
            "--start",
            "2026-02-11",
        ])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "remove", "2"])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "list"])
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

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "--interactive", "remove"])
        .write_stdin("1\n")
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("元日").not());
}

#[test]
fn add_interactive_cli() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();

    cmd()
        .args(["-f", file_str, "--interactive", "add"])
        .write_stdin("元日\n2026/1/1\n\n")
        .assert()
        .success();

    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-01-01 : 元日"));
}

#[test]
fn add_with_summary_and_start_does_not_prompt() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();

    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("End date").not());

    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1: 2026-01-01 : 元日"));
}

#[test]
fn list_sort_by_start_desc() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "憲法記念日",
            "--start",
            "2026-05-03",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "建国記念の日",
            "--start",
            "2026-02-11",
        ])
        .assert()
        .success();

    // Default order (insertion order)
    cmd()
        .args(["-f", file_str, "list"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1: 2026-01-01 : 元日"));

    // Sort by start ascending
    cmd()
        .args(["-f", file_str, "list", "--sort", "start"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1: 2026-01-01 : 元日"));

    // Sort by start descending
    cmd()
        .args(["-f", file_str, "list", "--sort", "start", "--desc"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1: 2026-05-03 : 憲法記念日"));
}

#[test]
fn list_sort_multi_key() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "B休日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "A休日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "C休日",
            "--start",
            "2026-02-01",
        ])
        .assert()
        .success();

    cmd()
        .args([
            "-f", file_str, "list", "--sort", "start", "--sort", "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1: 2026-01-01 : A休日"));
}

#[test]
fn file_option_after_subcommand_also_works() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    // --file after subcommand (clap global option supports this)
    cmd().args(["init", "-f", file_str]).assert().success();

    cmd()
        .args([
            "add",
            "-f",
            file_str,
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();

    cmd()
        .args(["list", "-f", file_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("元日"));
}

#[test]
fn list_json_output() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "元日",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success();

    let output = cmd()
        .args(["-f", file_str, "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    let arr = json.as_array().expect("JSON array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["summary"], "元日");
    assert_eq!(arr[0]["dtstart"], "2026-01-01");
    assert_eq!(arr[0]["dtend"], "2026-01-02");
}

#[test]
fn quiet_flag_suppresses_status_messages() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();

    // Without --quiet, add emits 'Added: ...' on stderr.
    cmd()
        .args([
            "-f",
            file_str,
            "add",
            "--summary",
            "noisy",
            "--start",
            "2026-01-01",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added: 2026-01-01"));

    // With --quiet (or -q), stderr stays empty for the status line.
    cmd()
        .args([
            "-f",
            file_str,
            "--quiet",
            "add",
            "--summary",
            "silent",
            "--start",
            "2026-01-02",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added: 2026-01-02").not());

    cmd()
        .args([
            "-f",
            file_str,
            "-q",
            "add",
            "--summary",
            "also-silent",
            "--start",
            "2026-01-03",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added:").not());
}

#[test]
fn no_interactive_in_non_tty_errors_when_args_missing() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.ics");
    let file_str = file.to_str().unwrap();

    cmd().args(["-f", file_str, "init"]).assert().success();

    // assert_cmd does not allocate a TTY for child processes; without
    // --interactive, the add subcommand sees no TTY and refuses to prompt.
    cmd()
        .args(["-f", file_str, "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no TTY"));
}
