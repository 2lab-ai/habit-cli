use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("habit").expect("binary built")
}

#[test]
fn add_list_roundtrip_json() {
    let td = tempfile::tempdir().unwrap();
    let db = td.path().join("db.json");

    bin()
        .env("HABITCLI_DB_PATH", &db)
        .env("HABITCLI_TODAY", "2026-01-31")
        .args([
            "add",
            "Stretch",
            "--schedule",
            "weekdays",
            "--target",
            "1",
            "--period",
            "day",
            "--format",
            "json",
        ])
        .assert()
        .success();

    bin()
        .env("HABITCLI_DB_PATH", &db)
        .args(["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stretch"));
}

#[test]
fn ambiguous_selector_exit_code_4() {
    let td = tempfile::tempdir().unwrap();
    let db = td.path().join("db.json");

    for name in ["Stretch", "Strength"] {
        bin()
            .env("HABITCLI_DB_PATH", &db)
            .env("HABITCLI_TODAY", "2026-01-31")
            .args(["add", name])
            .assert()
            .success();
    }

    bin()
        .env("HABITCLI_DB_PATH", &db)
        .args(["show", "str"])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("Ambiguous"));
}

#[test]
fn not_found_exit_code_3() {
    let td = tempfile::tempdir().unwrap();
    let db = td.path().join("db.json");

    bin()
        .env("HABITCLI_DB_PATH", &db)
        .args(["show", "h0001"])
        .assert()
        .failure()
        .code(3);
}
