//! CLI-level tests for the `gai` binary: argument parsing, subcommand
//! dispatch, and exit codes. Deliberately does NOT try to fake
//! /etc/nsswitch.conf, /etc/resolv.conf, or /etc/hosts (those paths are
//! hardcoded in gai_core::platform::paths and aren't injectable) — this
//! layer only checks that main.rs wires clap up correctly and that the
//! commands run to completion against whatever real config CI's Linux
//! runner has. Simulation logic itself is covered by gai-core's unit and
//! scenario tests, which run against real parsed fixtures.

use assert_cmd::Command;
use predicates::prelude::*;

fn gai() -> Command {
    Command::cargo_bin("gai").expect("gai binary should build")
}

#[test]
fn no_subcommand_fails_with_usage() {
    gai()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn help_lists_all_three_subcommands() {
    gai()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("why"));
}

#[test]
fn version_flag_prints_a_version() {
    gai()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gai"));
}

#[test]
fn explain_requires_a_name_argument() {
    gai().arg("explain").assert().failure();
}

#[test]
fn unknown_subcommand_is_rejected() {
    gai().arg("frobnicate").arg("localhost").assert().failure();
}

#[test]
fn explain_localhost_resolves_and_prints_the_chain() {
    // localhost is virtually always present in /etc/hosts on any Linux
    // box (including GitHub Actions runners), so this exercises the
    // real Files source end-to-end without needing network access.
    gai()
        .arg("explain")
        .arg("localhost")
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolution path"))
        .stdout(predicate::str::contains("Result:"));
}

#[test]
fn doctor_and_why_are_interchangeable_aliases() {
    // Both should reach the same code path and produce a DIAGNOSIS
    // section; doctor does live network I/O (DNS/mDNS reality check),
    // so this only asserts it completes and shapes its output — not
    // specific addresses, which would be flaky in CI.
    let doctor_out = gai()
        .arg("doctor")
        .arg("localhost")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let why_out = gai()
        .arg("why")
        .arg("localhost")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let doctor_out = String::from_utf8(doctor_out).unwrap();
    let why_out = String::from_utf8(why_out).unwrap();

    assert!(doctor_out.contains("DIAGNOSIS:"));
    assert!(why_out.contains("DIAGNOSIS:"));
}
