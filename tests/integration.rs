use std::env::current_dir;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Debug, PartialEq, Eq)]
struct StringOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

impl StringOutput {
    fn new(success: bool, stdout: &str, stderr: &str) -> Self {
        Self {
            success: success,
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
        }
    }
}

impl From<Output> for StringOutput {
    fn from(o: Output) -> Self {
        Self {
            success: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        }
    }
}

#[test]
fn workspace_all_match() {
    assert_eq!(
        StringOutput::new(true, "", ""),
        run_command_in_workspace("short").into()
    );
}

#[test]
fn workspace_some_match() {
    let want_stdout = format!(
        "{}\n",
        current_dir()
            .unwrap()
            .join("tests")
            .join("projects")
            .join("workspace_root")
            .join("wbin")
            .join("src")
            .join("main.rs")
            .display()
    );
    assert_eq!(
        StringOutput::new(false, &want_stdout, ""),
        run_command_in_workspace("long").into()
    );
}

#[test]
fn workspace_none_match() {
    let workspace_root = current_dir()
        .unwrap()
        .join("tests")
        .join("projects")
        .join("workspace_root");
    let files = vec![
        workspace_root.join("src").join("lib.rs"),
        workspace_root.join("wbin").join("src").join("main.rs"),
        workspace_root.join("wlib").join("src").join("lib.rs"),
    ];
    let want_stdout = files
        .into_iter()
        .map(|p| format!("{}\n", p.display()))
        .collect::<String>();
    assert_eq!(
        StringOutput::new(false, &want_stdout, ""),
        run_command_in_workspace("other").into()
    );
}

#[test]
fn workspace_all_wildcard_match() {
    assert_eq!(
        StringOutput::new(true, "", ""),
        run_command_in_workspace("wildcard").into()
    );
}

#[test]
fn file_too_short() {
    let output = run_command(&[
        "--prefix-path=tests/prefixes/really_long.txt",
        "--manifest-path=tests/projects/workspace_root/Cargo.toml",
        "-p",
        "wbin",
    ]);

    let want_stdout = format!(
        "{}\n",
        current_dir()
            .unwrap()
            .join("tests")
            .join("projects")
            .join("workspace_root")
            .join("wbin")
            .join("src")
            .join("main.rs")
            .display()
    );

    assert_eq!(StringOutput::new(false, &want_stdout, ""), output.into());
}

#[test]
fn package_not_found() {
    let output = run_command(&[
        "--prefix-path=tests/prefixes/short.txt",
        "--manifest-path=tests/projects/workspace_root/Cargo.toml",
        "-p",
        "doesnotexist",
    ]);
    assert_eq!(
        StringOutput::new(false, "", "Didn't find matching package(s)\n"),
        output.into()
    );
}

#[test]
fn manifest_file_not_found() {
    let output = run_command(&[
        "--prefix-path=tests/prefixes/short.txt",
        "--manifest-path=tests/projects/workspace_root/src/Cargo.toml",
    ]);
    assert_eq!(
        StringOutput::new(
            false,
            "",
            "Could not find tests/projects/workspace_root/src/Cargo.toml\n"
        ),
        output.into()
    );
}

#[test]
fn bad_manifest() {
    let path = "tests/projects/workspace_root/src/lib.rs";
    let output = run_command(&[
        format!("--prefix-path=tests/prefixes/short.txt"),
        format!("--manifest-path={}", path),
    ]);
    assert_eq!(
        StringOutput::new(
            false,
            "",
            &format!("Error parsing {}\n", abs(path).display())
        ),
        output.into()
    );
}

#[test]
fn prefix_file_not_found() {
    let path = "tests/prefixes/doesnotexist.txt";
    let output = run_command(&[
        format!("--prefix-path={}", path).as_str(),
        "--manifest-path=tests/projects/workspace_root/Cargo.toml",
    ]);
    assert_eq!(
        StringOutput::new(
            false,
            "",
            &format!("Error reading prefix-path file {}\n", path)
        ),
        output.into()
    );
}

#[test]
fn all_and_package() {
    let output = run_command(&[
        "--prefix-path=tests/prefixes/short.txt",
        "--manifest-path=tests/projects/workspace_root/Cargo.toml",
        "--all",
        "-p",
        "wlib",
    ]);
    assert_eq!(
        StringOutput::new(false, "", "Cannot specify --all and --package\n",),
        output.into(),
    );
}

fn run_command_in_workspace(prefix: &str) -> Output {
    run_command(&[
        format!("--prefix-path=tests/prefixes/{}.txt", prefix).as_str(),
        "--manifest-path=tests/projects/workspace_root/Cargo.toml",
        "--all",
    ])
}

fn run_command<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("target/debug/cargo-ensure-prefix")
        .args(args)
        .output()
        .unwrap()
}

fn abs(s: &str) -> PathBuf {
    current_dir().unwrap().join(s)
}
