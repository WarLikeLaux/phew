use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phew");
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/input/04_control_flow.php");

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(label: &str, contents: &str) -> Self {
        let path = std::env::temp_dir().join(format!("phew_cli_{}_{}.php", std::process::id(), label));
        std::fs::write(&path, contents).unwrap();
        Self { path }
    }

    fn arg(&self) -> &str {
        self.path.to_str().unwrap()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn run(args: &[&str], stdin: Option<&str>) -> (String, i32) {
    let mut command = Command::new(BIN);
    command.args(args);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn().unwrap();
    if let Some(data) = stdin {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(data.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    (stdout, output.status.code().unwrap())
}

fn formatted_fixture() -> String {
    let (stdout, code) = run(&[FIXTURE], None);
    assert_eq!(code, 0);
    stdout
}

#[test]
fn stdin_dash_formats_to_stdout() {
    let raw = std::fs::read_to_string(FIXTURE).unwrap();
    let expected = formatted_fixture();

    let (stdout, code) = run(&["-"], Some(&raw));

    assert_eq!(code, 0);
    assert_eq!(stdout, expected);
}

#[test]
fn check_succeeds_on_formatted_and_fails_on_unformatted() {
    let raw = std::fs::read_to_string(FIXTURE).unwrap();
    let formatted = formatted_fixture();
    assert_ne!(raw, formatted);

    let clean = TempFile::new("check_clean", &formatted);
    let dirty = TempFile::new("check_dirty", &raw);

    let (_, clean_code) = run(&["--check", clean.arg()], None);
    let (_, dirty_code) = run(&["--check", dirty.arg()], None);

    assert_eq!(clean_code, 0);
    assert_ne!(dirty_code, 0);
}

#[test]
fn check_writes_nothing() {
    let raw = std::fs::read_to_string(FIXTURE).unwrap();
    let dirty = TempFile::new("check_no_write", &raw);

    let (stdout, code) = run(&["--check", dirty.arg()], None);

    assert_ne!(code, 0);
    assert!(stdout.is_empty());
    assert_eq!(std::fs::read_to_string(&dirty.path).unwrap(), raw);
}

#[test]
fn diff_shows_hunks_for_unformatted() {
    let raw = std::fs::read_to_string(FIXTURE).unwrap();
    let dirty = TempFile::new("diff_dirty", &raw);

    let (stdout, code) = run(&["--diff", dirty.arg()], None);

    assert_eq!(code, 0);
    assert!(stdout.contains("@@"));
    assert!(stdout.contains('+'));
    assert!(stdout.contains('-'));
    assert_eq!(std::fs::read_to_string(&dirty.path).unwrap(), raw);
}

#[test]
fn diff_is_empty_for_formatted() {
    let formatted = formatted_fixture();
    let clean = TempFile::new("diff_clean", &formatted);

    let (stdout, code) = run(&["--diff", clean.arg()], None);

    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn missing_file_fails() {
    let missing = std::env::temp_dir()
        .join(format!("phew_cli_missing_{}_file.php", std::process::id()))
        .to_string_lossy()
        .into_owned();

    let (_, code) = run(&[&missing], None);

    assert_ne!(code, 0);
}

#[test]
fn conflicting_modes_are_rejected() {
    let (_, code) = run(&["--write", "--check", FIXTURE], None);

    assert_ne!(code, 0);
}
