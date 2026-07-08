use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_eolify")
}

fn temp_dir() -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("eolify-cli-test-{}-{id}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}

#[test]
fn check_lf_accepts_lf_and_no_newline_rejects_crlf_and_cr() {
    let dir = temp_dir();
    let lf = dir.join("lf.txt");
    let none = dir.join("none.txt");
    let crlf = dir.join("crlf.txt");
    let cr = dir.join("cr.txt");
    write(&lf, b"a\nb\n");
    write(&none, b"abc");
    write(&crlf, b"a\r\nb\r\n");
    write(&cr, b"a\rb\r");

    let ok = Command::new(bin())
        .args(["check", "--to", "lf"])
        .arg(&lf)
        .arg(&none)
        .status()
        .unwrap();
    assert!(ok.success());

    let bad = Command::new(bin())
        .args(["check", "--to", "lf"])
        .arg(&crlf)
        .arg(&cr)
        .status()
        .unwrap();
    assert_eq!(bad.code(), Some(1));
}

#[test]
fn check_crlf_accepts_crlf_and_no_newline_rejects_lf_and_cr() {
    let dir = temp_dir();
    let crlf = dir.join("crlf.txt");
    let none = dir.join("none.txt");
    let lf = dir.join("lf.txt");
    let cr = dir.join("cr.txt");
    write(&crlf, b"a\r\nb\r\n");
    write(&none, b"abc");
    write(&lf, b"a\nb\n");
    write(&cr, b"a\rb\r");

    let ok = Command::new(bin())
        .args(["check", "--to", "crlf"])
        .arg(&crlf)
        .arg(&none)
        .status()
        .unwrap();
    assert!(ok.success());

    let bad = Command::new(bin())
        .args(["check", "--to", "crlf"])
        .arg(&lf)
        .arg(&cr)
        .status()
        .unwrap();
    assert_eq!(bad.code(), Some(1));
}

#[test]
fn check_mixed_fails_only_mixed_files() {
    let dir = temp_dir();
    let lf = dir.join("lf.txt");
    let crlf = dir.join("crlf.txt");
    let mixed = dir.join("mixed.txt");
    write(&lf, b"a\nb\n");
    write(&crlf, b"a\r\nb\r\n");
    write(&mixed, b"a\nb\r\n");

    let ok = Command::new(bin())
        .args(["check", "--mixed"])
        .arg(&lf)
        .arg(&crlf)
        .status()
        .unwrap();
    assert!(ok.success());

    let bad = Command::new(bin())
        .args(["check", "--mixed"])
        .arg(&mixed)
        .status()
        .unwrap();
    assert_eq!(bad.code(), Some(1));
}

#[test]
fn check_requires_exactly_one_policy() {
    let dir = temp_dir();
    let file = dir.join("file.txt");
    write(&file, b"a\n");

    let missing = Command::new(bin())
        .arg("check")
        .arg(&file)
        .status()
        .unwrap();
    assert_eq!(missing.code(), Some(2));

    let conflicting = Command::new(bin())
        .args(["check", "--to", "lf", "--mixed"])
        .arg(&file)
        .status()
        .unwrap();
    assert_eq!(conflicting.code(), Some(2));
}

#[test]
fn stats_aggregates_argv_and_files_from() {
    let dir = temp_dir();
    let a = dir.join("a.txt");
    let b = dir.join("b.txt");
    let list = dir.join("files.txt");
    write(&a, b"a\nb\r\n");
    write(&b, b"c\rd\r\n");
    write(&list, format!("{}\n", b.display()).as_bytes());

    let output = Command::new(bin())
        .args(["stats", "--files-from"])
        .arg(&list)
        .arg(&a)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("total: lf=1 crlf=2 cr=1"), "{stdout}");
}

#[test]
fn stats_files_prints_per_file_lines() {
    let dir = temp_dir();
    let a = dir.join("a.txt");
    write(&a, b"a\nb\r\n");

    let output = Command::new(bin())
        .args(["stats", "--files"])
        .arg(&a)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&format!("{}: lf=1 crlf=1 cr=0", a.display())),
        "{stdout}"
    );
    assert!(stdout.contains("total: lf=1 crlf=1 cr=0"), "{stdout}");
}

#[test]
fn json_output_is_valid_and_escaped() {
    let dir = temp_dir();
    let quoted = dir.join("quote\"line\nname.txt");
    write(&quoted, b"a\r\nb\n");

    let check = Command::new(bin())
        .args(["check", "--to", "lf", "--json"])
        .arg(&quoted)
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    let check_json: Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(
        check_json[0]["path"].as_str().unwrap(),
        quoted.to_string_lossy()
    );
    assert_eq!(check_json[0]["status"], "nonconforming");
    assert_eq!(check_json[0]["lf"], 1);
    assert_eq!(check_json[0]["crlf"], 1);

    let stats = Command::new(bin())
        .args(["stats", "--json"])
        .arg(&quoted)
        .output()
        .unwrap();
    assert!(stats.status.success());
    let stats_json: BTreeMap<String, Value> = serde_json::from_slice(&stats.stdout).unwrap();
    assert_eq!(
        stats_json["files"][0]["path"].as_str().unwrap(),
        quoted.to_string_lossy()
    );
    assert_eq!(stats_json["total"]["lf"], 1);
    assert_eq!(stats_json["total"]["crlf"], 1);
}

#[test]
fn files_from_stdin_null_handles_paths_with_spaces_and_newlines() {
    let dir = temp_dir();
    let spaced = dir.join("space name.txt");
    let newline = dir.join("line\nname.txt");
    write(&spaced, b"a\n");
    write(&newline, b"b\n");

    let mut child = Command::new(bin())
        .args(["check", "--mixed", "--files-from", "-", "--null"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(spaced.to_string_lossy().as_bytes())
            .unwrap();
        stdin.write_all(&[0]).unwrap();
        stdin
            .write_all(newline.to_string_lossy().as_bytes())
            .unwrap();
        stdin.write_all(&[0]).unwrap();
    }

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn fix_lf_rewrites_only_changed_files() {
    let dir = temp_dir();
    let lf = dir.join("lf.txt");
    let crlf = dir.join("crlf.txt");
    write(&lf, b"a\n");
    write(&crlf, b"a\r\n");
    let before = fs::metadata(&lf).unwrap().modified().unwrap();

    let status = Command::new(bin())
        .args(["fix", "--to", "lf"])
        .arg(&lf)
        .arg(&crlf)
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(fs::read(&lf).unwrap(), b"a\n");
    assert_eq!(fs::read(&crlf).unwrap(), b"a\n");
    assert_eq!(fs::metadata(&lf).unwrap().modified().unwrap(), before);
}

#[test]
fn binary_and_utf16_are_skipped_by_default_and_fail_when_requested() {
    let dir = temp_dir();
    let binary = dir.join("binary.bin");
    let utf16 = dir.join("utf16.txt");
    write(&binary, b"\0PNG\r\n");
    write(&utf16, b"\xff\xfea\0\r\0\n\0");

    let skipped = Command::new(bin())
        .args(["check", "--to", "lf"])
        .arg(&binary)
        .arg(&utf16)
        .status()
        .unwrap();
    assert!(skipped.success());

    let failed = Command::new(bin())
        .args(["check", "--to", "lf", "--unsupported", "fail"])
        .arg(&binary)
        .arg(&utf16)
        .status()
        .unwrap();
    assert_eq!(failed.code(), Some(2));
}

#[test]
fn directories_are_skipped_by_default_and_fail_when_requested() {
    let dir = temp_dir();
    let subdir = dir.join("subdir");
    fs::create_dir(&subdir).unwrap();

    let skipped = Command::new(bin())
        .args(["check", "--to", "lf"])
        .arg(&subdir)
        .status()
        .unwrap();
    assert!(skipped.success());

    let failed = Command::new(bin())
        .args(["check", "--to", "lf", "--unsupported", "fail"])
        .arg(&subdir)
        .status()
        .unwrap();
    assert_eq!(failed.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn symlinked_directories_are_skipped_by_default() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir();
    let target = dir.join("target");
    let link = dir.join("link");
    fs::create_dir(&target).unwrap();
    symlink(&target, &link).unwrap();

    let skipped = Command::new(bin())
        .args(["check", "--to", "lf"])
        .arg(&link)
        .status()
        .unwrap();
    assert!(skipped.success());
}
