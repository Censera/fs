use fsize_core::{Unit, format_size};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new() -> Self {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let id = NEXT.fetch_add(1, Ordering::Relaxed);

        let path = std::env::temp_dir().join(format!("fsize-test-{}-{id}", time.as_nanos()));

        fs::create_dir(&path).unwrap();

        Self { path }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn file(path: &Path, size: usize) {
    fs::write(path, vec![0u8; size]).unwrap();
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fsize"))
        .args(args)
        .output()
        .unwrap()
}

fn out(result: &Output) -> String {
    String::from_utf8(result.stdout.clone()).unwrap()
}

fn err(result: &Output) -> String {
    String::from_utf8(result.stderr.clone()).unwrap()
}

#[test]
fn regular_file() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 1234);

    let result = run(&["--raw", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1234\n");
}

#[test]
fn recursive_dir() {
    let tmp = Temp::new();

    file(&tmp.path.join("a"), 100);
    file(&tmp.path.join("b"), 200);

    let sub = tmp.path.join("sub");
    fs::create_dir(&sub).unwrap();
    file(&sub.join("c"), 300);

    let result = run(&["--raw", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "600\n");
}

#[test]
fn empty_dir() {
    let tmp = Temp::new();

    let result = run(&["--raw", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "0\n");
}

#[test]
fn nonexistent_path() {
    let tmp = Temp::new();
    let path = tmp.path.join("missing");

    let result = run(&["--raw", path.to_str().unwrap()]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[cfg(unix)]
#[test]
fn unreadable_entry() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = Temp::new();

    file(&tmp.path.join("visible"), 100);

    let hidden = tmp.path.join("hidden");
    fs::create_dir(&hidden).unwrap();
    file(&hidden.join("secret"), 200);

    fs::set_permissions(&hidden, fs::Permissions::from_mode(0o000)).unwrap();

    let result = run(&["--raw", tmp.path.to_str().unwrap()]);

    fs::set_permissions(&hidden, fs::Permissions::from_mode(0o755)).unwrap();

    let stdout = out(&result);

    assert!(
        stdout == "100\n" || stdout == "300\n",
        "unexpected output: {stdout:?}"
    );

    if stdout == "100\n" {
        assert!(
            err(&result).contains("warning:"),
            "expected warning, got: {:?}",
            err(&result)
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinks_lflag() {
    use std::os::unix::fs::symlink;

    let tmp = Temp::new();

    file(&tmp.path.join("file"), 100);

    let a = tmp.path.join("a");
    let b = tmp.path.join("b");

    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();

    file(&a.join("one"), 200);
    file(&b.join("two"), 300);

    symlink("../file", a.join("link")).unwrap();

    let result = run(&["--raw", "-L", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "900\n");
}

#[cfg(unix)]
#[test]
fn symlinks_nonlflag() {
    use std::os::unix::fs::symlink;

    let tmp = Temp::new();

    file(&tmp.path.join("file"), 100);
    symlink("file", tmp.path.join("link")).unwrap();

    let result = run(&["--raw", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "100\n");
}

#[cfg(unix)]
#[test]
fn symlinks_cycles() {
    use std::os::unix::fs::symlink;

    let tmp = Temp::new();

    let a = tmp.path.join("a");
    let b = tmp.path.join("b");

    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();

    file(&a.join("one"), 100);
    file(&b.join("two"), 200);

    symlink("../", a.join("loop")).unwrap();

    let result = run(&["--raw", "-L", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "300\n");
    assert!(err(&result).contains("already-visited"));
}

#[cfg(unix)]
#[test]
fn hardlinks() {
    use std::fs::hard_link;

    let tmp = Temp::new();

    let a = tmp.path.join("a");
    let b = tmp.path.join("b");

    file(&a, 1234);
    hard_link(&a, &b).unwrap();

    let result = run(&["--raw", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1234\n");
}

#[test]
fn max_depth() {
    let tmp = Temp::new();

    file(&tmp.path.join("root"), 10);

    let one = tmp.path.join("one");
    fs::create_dir(&one).unwrap();
    file(&one.join("one"), 20);

    let two = one.join("two");
    fs::create_dir(&two).unwrap();
    file(&two.join("two"), 30);

    let result = run(&["--raw", "--max-depth", "1", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "10\n");

    let result = run(&["--raw", "--max-depth", "2", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "30\n");
}

#[test]
fn exclusions() {
    let tmp = Temp::new();

    file(&tmp.path.join("keep"), 100);
    file(&tmp.path.join("skip.txt"), 200);
    file(&tmp.path.join("skip.log"), 300);

    let sub = tmp.path.join("sub");
    fs::create_dir(&sub).unwrap();
    file(&sub.join("keep"), 400);

    let result = run(&[
        "--raw",
        "--exclude",
        "*.txt|*.log",
        tmp.path.to_str().unwrap(),
    ]);

    assert!(result.status.success());
    assert_eq!(out(&result), "500\n");
}

#[test]
fn multiple_paths() {
    let tmp = Temp::new();

    let a = tmp.path.join("a");
    let b = tmp.path.join("b");

    file(&a, 10);
    file(&b, 20);

    let result = run(&["--raw", a.to_str().unwrap(), b.to_str().unwrap()]);

    assert!(result.status.success());

    let stdout = out(&result);
    let lines: Vec<_> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("a"));
    assert!(lines[0].contains("10"));
    assert!(lines[1].contains("b"));
    assert!(lines[1].contains("20"));
}

#[test]
fn metadata_flag() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 1234);

    let result = run(&["--raw", "--metadata", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1234\n");
}

#[test]
fn binary_flag() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 1536);

    let result = run(&["--binary", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1.5 KiB\n");
}

#[test]
fn raw_flag() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 1536);

    let result = run(&["--raw", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1536\n");
}

#[test]
fn unit_flag() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 2048);

    let result = run(&["--unit", "KB", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "2.05 KB\n");
}

#[test]
fn json_file() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 1234);

    let result = run(&["--json", path.to_str().unwrap()]);

    assert!(result.status.success());

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();

    assert_eq!(json["bytes"], "1234");
    assert_eq!(json["formatted"], "1.23 KB");
}

#[cfg(unix)]
#[test]
fn disk_usage() {
    let result = run(&["--disk-usage"]);

    assert!(result.status.success());

    let stdout = out(&result);

    assert!(stdout.contains("total"));
    assert!(stdout.contains("used"));
    assert!(stdout.contains("free"));
    assert!(stdout.contains("available"));
}

#[test]
fn disk_usage_json() {
    let result = run(&["--disk-usage", "--json"]);

    assert!(result.status.success());

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();

    let total = json["total"].as_str().unwrap().parse::<u128>().unwrap();

    let used = json["used"].as_str().unwrap().parse::<u128>().unwrap();

    let free = json["free"].as_str().unwrap().parse::<u128>().unwrap();

    let available = json["available"].as_str().unwrap().parse::<u128>().unwrap();

    assert!(total > 0);
    assert_eq!(used, total.saturating_sub(free));
    assert!(available <= free);
}

#[test]
fn invalid_binary_raw() {
    let result = run(&["-br"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn invalid_binary_unit() {
    let result = run(&["-bu", "GB"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn invalid_raw_unit() {
    let result = run(&["-ru", "GB"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn invalid_metadata_symlinks() {
    let result = run(&["-mL"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn invalid_metadata_exclude() {
    let result = run(&["-m", "--exclude", "*.txt"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn invalid_metadata_depth() {
    let result = run(&["-m", "--max-depth", "2"]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn disk_no_path() {
    let tmp = Temp::new();

    let result = run(&["--disk-usage", tmp.path.to_str().unwrap()]);

    assert!(!result.status.success());
    assert!(err(&result).contains("error:"));
}

#[test]
fn disk_exclusive() {
    let cases = [
        vec!["-d", "-m"],
        vec!["-d", "-L"],
        vec!["-d", "-b"],
        vec!["-d", "-r"],
        vec!["-d", "-u", "GB"],
        vec!["-d", "--exclude", "*.txt"],
        vec!["-d", "--max-depth", "2"],
    ];

    for args in cases {
        let result = run(&args);

        assert!(
            !result.status.success(),
            "accepted invalid arguments: {args:?}"
        );

        assert!(
            err(&result).contains("error:"),
            "missing error for: {args:?}"
        );
    }
}

#[test]
fn help() {
    let result = run(&["--help"]);

    assert!(result.status.success());

    let stdout = out(&result);

    assert!(stdout.contains("fsize"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("Examples:"));
}

#[test]
fn version() {
    let result = run(&["--version"]);

    assert!(result.status.success());

    let stdout = out(&result);

    assert!(stdout.starts_with("fsize MOLE "));
}

#[test]
fn end_of_options() {
    let tmp = Temp::new();
    let path = tmp.path.join("-file");

    file(&path, 1234);

    let result = run(&["--raw", "--", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "1234\n");
}

#[test]
fn exclude_multiple_patterns() {
    let tmp = Temp::new();

    file(&tmp.path.join("keep"), 100);
    file(&tmp.path.join("one.tmp"), 200);
    file(&tmp.path.join("two.log"), 300);
    file(&tmp.path.join("three"), 400);

    let result = run(&[
        "--raw",
        "--exclude",
        "*.tmp|*.log",
        tmp.path.to_str().unwrap(),
    ]);

    assert!(result.status.success());
    assert_eq!(out(&result), "500\n");
}

#[test]
fn exclude_attached() {
    let tmp = Temp::new();

    file(&tmp.path.join("keep"), 100);
    file(&tmp.path.join("skip"), 200);

    let result = run(&["--raw", "--exclude=skip", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "100\n");
}

#[test]
fn max_depth_attached() {
    let tmp = Temp::new();

    file(&tmp.path.join("root"), 10);

    let sub = tmp.path.join("sub");
    fs::create_dir(&sub).unwrap();
    file(&sub.join("file"), 20);

    let result = run(&["--raw", "--max-depth=1", tmp.path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "10\n");
}

#[test]
fn unit_attached() {
    let tmp = Temp::new();
    let path = tmp.path.join("file");

    file(&path, 2048);

    let result = run(&["--unit=KB", path.to_str().unwrap()]);

    assert!(result.status.success());
    assert_eq!(out(&result), "2.05 KB\n");
}

#[test]
fn unit_formatting() {
    assert_eq!(format_size(0, None, false), "0 B");

    assert_eq!(format_size(123, None, false), "123 B");

    assert_eq!(format_size(1500, None, false), "1.5 KB");

    assert_eq!(format_size(1536, None, true), "1.5 KiB");

    assert_eq!(format_size(2048, Some(Unit::KB), false), "2.05 KB");

    assert_eq!(format_size(2048, Some(Unit::KiB), false), "2 KiB");

    assert_eq!(format_size(1_000_000, Some(Unit::MB), false), "1 MB");

    assert_eq!(format_size(1_048_576, Some(Unit::MiB), false), "1 MiB");
}
