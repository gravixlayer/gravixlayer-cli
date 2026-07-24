//! CLI smoke tests — no network required.
//!
//! These use `assert_cmd` against the compiled `gravixlayer` binary to catch
//! regressions in help/version/dispatch wiring before a release.

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;

fn gravixlayer() -> Command {
    Command::new(cargo_bin!("gravixlayer"))
}

#[test]
fn version_flag_prints_package_version() {
    gravixlayer()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_core_commands() {
    gravixlayer()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("runtime"))
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("template"));
}

#[test]
fn runtime_help_lists_shell_and_exec() {
    gravixlayer()
        .args(["runtime", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shell"))
        .stdout(predicate::str::contains("exec"));
}

#[test]
fn update_help_documents_check_and_version() {
    gravixlayer()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--version"));
}

#[test]
fn completions_bash_emits_script() {
    gravixlayer()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gravixlayer"));
}
