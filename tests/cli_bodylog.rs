//! Integration tests for bodylog integration (spec/03-bodylog.md Phases 2/3/6).
//! Uses a temporary fake bodylog binary (script) to avoid depending on the real tool.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::tempdir;

/// Create a fake `bodylog` executable in a temp dir that prints canned --json responses
/// for the subcommands we care about (measurement list, report weight/summary, config show).
fn make_fake_bodylog(dir: &tempfile::TempDir) -> PathBuf {
    let script_path = dir.path().join("bodylog");
    let mut f = fs::File::create(&script_path).expect("create fake bodylog");
    // A small POSIX shell script that inspects $1 $2 etc and echoes known JSON.
    // It ignores most flags and always succeeds with --json shaped output.
    let script = r#"#!/bin/sh
# Fake bodylog for krebslog tests
set -e
# Consume --json if present (we always emit JSON)
shift || true
while [ $# -gt 0 ]; do
  case "$1" in
    --json) shift; continue ;;
    --since|--until|--days) shift; shift; continue ;;
    *) break ;;
  esac
done

cmd="$1"
sub="$2"

case "$cmd $sub" in
  "measurement list")
    # Return a small newest-first array with two days
    printf '{"id":2,"date":"2026-06-07","weight_kg":82.1,"body_fat_pct":21.1,"skeletal_muscle_pct":37.6,"visceral_fat_level":10,"bmi":26.8}\n'
    printf '{"id":1,"date":"2026-06-01","weight_kg":82.7,"body_fat_pct":21.4,"skeletal_muscle_pct":37.4,"visceral_fat_level":10,"bmi":26.9}\n'
    ;;
  "report weight"|"report summary")
    # Minimal report shape with stats + series (series is what we cache)
    printf '{"period":{"since":"2026-06-01"},"weight":{"count":2,"min":82.1,"max":82.7,"avg":82.4,"start":82.7,"end":82.1,"change":-0.6,"trend":"down"},"series":[{"date":"2026-06-07","weight_kg":82.1},{"date":"2026-06-01","weight_kg":82.7}],"measurement_count":2}\n'
    ;;
  "config show")
    printf '{"height_cm":175.0,"date_of_birth":"1983-07-21","updated_at":{"utc":"2026-06-08T00:00:00Z"}}\n'
    ;;
  *)
    # Unknown but don't crash the test harness
    printf '[]\n'
    ;;
esac
"#;
    f.write_all(script.as_bytes()).expect("write fake script");
    let mut perms = fs::metadata(&script_path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("chmod +x fake bodylog");
    script_path
}

#[test]
fn data_pull_bodylog_with_override_produces_real_shapes() {
    let tmp = tempdir().expect("tempdir");
    let fake = make_fake_bodylog(&tmp);

    let mut cmd = Command::cargo_bin("krebslog").expect("bin");
    cmd.arg("--json")
        .arg("data")
        .arg("pull")
        .arg("--source")
        .arg("bodylog")
        .arg("--entity")
        .arg("measurement")
        .arg("--since")
        .arg("last 7 days")
        .arg("--bodylog-bin")
        .arg(fake.to_str().unwrap());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("82.1"))
        .stdout(predicate::str::contains("skeletal_muscle_pct"));
}

#[test]
fn data_status_probe_lists_bodylog_when_override_present() {
    let tmp = tempdir().expect("tempdir");
    let fake = make_fake_bodylog(&tmp);

    let mut cmd = Command::cargo_bin("krebslog").expect("bin");
    cmd.arg("--json")
        .arg("data")
        .arg("status")
        .arg("--probe")
        .arg("--bodylog-bin")
        .arg(fake.to_str().unwrap());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"source\": \"bodylog\""))
        .stdout(predicate::str::contains("\"available\": true"))
        .stdout(predicate::str::contains("probe"));
}

#[test]
fn report_energy_balance_include_body_trends_uses_override_and_emits_body_block() {
    let tmp = tempdir().expect("tempdir");
    let fake = make_fake_bodylog(&tmp);

    let mut cmd = Command::cargo_bin("krebslog").expect("bin");
    cmd.arg("--json")
        .arg("report")
        .arg("energy-balance")
        .arg("--since")
        .arg("last 7 days")
        .arg("--include-body-trends")
        .arg("--bodylog-bin")
        .arg(fake.to_str().unwrap());

    // Should succeed and contain a body block with either stats or series from the fake
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"body\""))
        .stdout(predicate::str::contains("82.1").or(predicate::str::contains("change")));
}
