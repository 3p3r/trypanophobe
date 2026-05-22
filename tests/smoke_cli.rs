use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("trypanophobe").unwrap()
}

#[test]
fn smoke_version() {
    bin().arg("version").assert().success().stdout(predicate::str::contains(
        "trypanophobe",
    ));
}

#[test]
fn smoke_check_rejects_non_english_fast() {
    bin().arg("check")
        .arg("Bonjour le monde")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn smoke_help() {
    bin().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("check"));
}
