use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

fn habit_cmd() -> Command {
    Command::cargo_bin("habit").expect("binary habit is built")
}

fn read_json(stdout: &[u8]) -> Value {
    serde_json::from_slice(stdout).expect("valid json")
}

#[test]
fn add_list_show_flow_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.json");

    // Add Stretch
    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "--format",
            "json",
            "add",
            "Stretch",
            "--schedule",
            "weekdays",
            "--target",
            "1",
            "--period",
            "day",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["habit"]["id"], "h0001");

    // Add Read
    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "--format",
            "json",
            "add",
            "Read",
            "--schedule",
            "everyday",
            "--target",
            "1",
            "--period",
            "day",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["habit"]["id"], "h0002");

    // List (sorted by name)
    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "json",
            "list",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    let names: Vec<String> = v["habits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["Read", "Stretch"]);

    // Show via unique name prefix
    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "json",
            "show",
            "str",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["habit"]["id"], "h0001");
    assert_eq!(v["habit"]["name"], "Stretch");
}

#[test]
fn ambiguous_selector_exit_code_4() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.json");

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "add",
            "Stretch",
        ])
        .assert()
        .success();

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "add",
            "Study",
        ])
        .assert()
        .success();

    habit_cmd()
        .args(["--db", db.to_str().unwrap(), "show", "st"])
        .assert()
        .failure()
        .code(4)
        .stderr(
            predicate::str::contains("ambiguous selector")
                .and(predicate::str::contains("candidates")),
        );
}

#[test]
fn checkin_add_set_delete() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.json");

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "add",
            "Stretch",
        ])
        .assert()
        .success();

    // default qty=1
    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "checkin",
            "stretch",
        ])
        .assert()
        .success();

    // add +2
    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "checkin",
            "stretch",
            "--qty",
            "2",
        ])
        .assert()
        .success();

    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "json",
            "show",
            "stretch",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["checkins"].as_array().unwrap().len(), 1);
    assert_eq!(v["checkins"][0]["quantity"], 3);

    // set to 1
    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "checkin",
            "stretch",
            "--set",
            "1",
        ])
        .assert()
        .success();

    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "json",
            "show",
            "stretch",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["checkins"][0]["quantity"], 1);

    // delete
    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "checkin",
            "stretch",
            "--delete",
        ])
        .assert()
        .success();

    let out = habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "json",
            "show",
            "stretch",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["checkins"].as_array().unwrap().len(), 0);
}

#[test]
fn archive_and_list_visibility() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.json");

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "add",
            "Read",
        ])
        .assert()
        .success();

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-02-01",
            "archive",
            "read",
        ])
        .assert()
        .success();

    // list excludes archived by default
    let out = habit_cmd()
        .args(["--db", db.to_str().unwrap(), "--format", "json", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["habits"].as_array().unwrap().len(), 0);

    // list --all includes archived
    let out = habit_cmd()
        .args(["--db", db.to_str().unwrap(), "--format", "json", "list", "--all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["habits"].as_array().unwrap().len(), 1);
    assert_eq!(v["habits"][0]["archived"], true);
}

#[test]
fn export_json_and_csv() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.json");

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "add",
            "Stretch",
        ])
        .assert()
        .success();

    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--today",
            "2026-01-31",
            "checkin",
            "stretch",
        ])
        .assert()
        .success();

    // JSON to stdout
    let out = habit_cmd()
        .args(["--db", db.to_str().unwrap(), "--format", "json", "export"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = read_json(&out);
    assert_eq!(v["version"], 1);
    assert_eq!(v["habits"].as_array().unwrap().len(), 1);
    assert_eq!(v["checkins"].as_array().unwrap().len(), 1);

    // CSV to directory
    let out_dir = dir.path().join("export");
    habit_cmd()
        .args([
            "--db",
            db.to_str().unwrap(),
            "--format",
            "csv",
            "export",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let habits_csv = fs::read_to_string(out_dir.join("habits.csv")).unwrap();
    assert!(habits_csv
        .lines()
        .next()
        .unwrap()
        .contains("id,name,schedule"));

    let checkins_csv = fs::read_to_string(out_dir.join("checkins.csv")).unwrap();
    assert!(checkins_csv
        .lines()
        .next()
        .unwrap()
        .contains("habit_id,date,quantity"));
}
