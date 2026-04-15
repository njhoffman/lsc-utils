//! End-to-end CLI tests using assert_cmd. Pin COLUMNS, force --color=never,
//! and use isolated TempDirs so output is fully deterministic.

use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use predicates::str;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("alpha.txt"), "").unwrap();
    std::fs::write(tmp.path().join("bravo.rs"), "").unwrap();
    std::fs::write(tmp.path().join(".dotfile"), "").unwrap();
    std::fs::create_dir(tmp.path().join("subdir")).unwrap();
    tmp
}

fn lsc() -> Command {
    let mut c = Command::cargo_bin("lsc").unwrap();
    c.env("COLUMNS", "80")
        .env_remove("DEBUG")
        .env_remove("NO_COLOR");
    c
}

#[test]
fn one_per_line_lists_visible_entries() {
    let tmp = fixture();
    lsc()
        .args(["-1", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(str::contains("alpha.txt"))
        .stdout(str::contains("bravo.rs"))
        .stdout(str::contains("subdir"))
        .stdout(str::contains(".dotfile").not());
}

#[test]
fn all_flag_includes_dotfile_dot_dotdot() {
    let tmp = fixture();
    let assert = lsc()
        .args(["-a", "-1", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success();
    let lines: Vec<&str> = std::str::from_utf8(&assert.get_output().stdout)
        .unwrap()
        .lines()
        .collect();
    assert!(lines.contains(&"."), "missing `.` in {lines:?}");
    assert!(lines.contains(&".."), "missing `..` in {lines:?}");
    assert!(
        lines.contains(&".dotfile"),
        "missing `.dotfile` in {lines:?}"
    );
}

#[test]
fn almost_all_includes_dotfile_but_not_dot() {
    let tmp = fixture();
    let assert = lsc()
        .args(["-A", "-1", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains(".dotfile"));
    assert!(!stdout.contains("\n.\n"));
    assert!(!stdout.contains("\n..\n"));
}

#[test]
fn only_dirs_filters_out_files() {
    let tmp = fixture();
    let assert = lsc()
        .args(["-d", "-1", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("subdir"));
    assert!(!stdout.contains("alpha.txt"));
}

#[test]
fn only_files_filters_out_dirs() {
    let tmp = fixture();
    let assert = lsc()
        .args(["-f", "-1", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("alpha.txt"));
    assert!(!stdout.contains("subdir"));
}

#[test]
fn vertical_grid_fits_columns() {
    let tmp = TempDir::new().unwrap();
    for n in &["aa", "bb", "cc", "dd", "ee", "ff"] {
        std::fs::write(tmp.path().join(n), "").unwrap();
    }
    let out = lsc()
        .args(["-C", "--color=never", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    // Six 2-char names with gap=2 fit on one row of 80 cols, but vertical
    // layout fills column-major. With 80 cols available, all 6 entries fit
    // on one row (cols=6, rows=1).
    let line_count = s.trim_end().lines().count();
    assert_eq!(line_count, 1, "expected 1 row, got: {s:?}");
}

#[test]
fn color_always_emits_sgr() {
    let tmp = fixture();
    let out = lsc()
        .args(["-1", "--color=always", "--without-icons"])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("\x1b["), "expected ANSI SGR in output: {s:?}");
}

#[test]
fn explicit_file_path_renders_one_entry() {
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("only.rs");
    std::fs::write(&p, "").unwrap();
    let out = lsc()
        .args(["-1", "--color=never", "--without-icons"])
        .arg(&p)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(out).unwrap(), "only.rs\n");
}

#[test]
fn multiple_targets_get_headers() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    std::fs::write(a.path().join("xa"), "").unwrap();
    std::fs::write(b.path().join("xb"), "").unwrap();
    let out = lsc()
        .args(["-1", "--color=never", "--without-icons"])
        .arg(a.path())
        .arg(b.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains(&format!("{}:", a.path().display())));
    assert!(s.contains(&format!("{}:", b.path().display())));
    assert!(s.contains("xa"));
    assert!(s.contains("xb"));
}

#[test]
fn nonexistent_path_errors() {
    lsc()
        .arg("/no/such/path/at/all/zzzzz")
        .assert()
        .failure()
        .stderr(str::contains("lsc:"));
}

#[test]
fn version_flag() {
    lsc().arg("--version").assert().success();
}
