use std::process::Command;

fn cargo_avail() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cargo-avail"))
}

#[test]
fn no_args_exits_with_code_2() {
    let output = cargo_avail().output().expect("failed to execute");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no crate names provided"),
        "stderr: {stderr}"
    );
}

#[test]
fn invalid_name_exits_with_code_1() {
    let output = cargo_avail()
        .arg("foo+bar")
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("invalid"), "stdout: {stdout}");
}

#[test]
fn reserved_name_exits_with_code_1() {
    let output = cargo_avail()
        .arg("std")
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reserved"), "stdout: {stdout}");
}

#[test]
fn quiet_flag_suppresses_stdout() {
    let output = cargo_avail()
        .args(["--quiet", "std"])
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty in quiet mode"
    );
}

#[test]
fn tab_separated_output_format() {
    let output = cargo_avail()
        .arg("std")
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\t'),
        "output should be tab-separated: {stdout}"
    );
}

#[test]
fn multiple_names_all_checked() {
    let output = cargo_avail()
        .args(["std", "core", "foo+bar"])
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("std"), "should contain std: {stdout}");
    assert!(stdout.contains("core"), "should contain core: {stdout}");
    assert!(
        stdout.contains("foo+bar"),
        "should contain foo+bar: {stdout}"
    );
}

#[test]
fn stdin_piping() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-avail"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(b"std\ncore\n").ok();
            }
            child.wait_with_output()
        })
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("std"),
        "stdin: should contain std: {stdout}"
    );
    assert!(
        stdout.contains("core"),
        "stdin: should contain core: {stdout}"
    );
}

#[test]
fn deduplicates_canonical_names() {
    // foo-bar and foo_bar are canonically the same; should only appear once
    let output = cargo_avail()
        .args(["foo-bar", "foo_bar"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // Should deduplicate to just one entry (the first occurrence)
    let foo_lines: Vec<&&str> = lines.iter().filter(|l| l.starts_with("foo")).collect();
    assert_eq!(foo_lines.len(), 1, "should deduplicate: {stdout}");
}

#[test]
fn hyphen_valued_names_accepted() {
    // Names starting with hyphens should be accepted as arguments (invalid crate names,
    // but should not be treated as flags)
    let output = cargo_avail()
        .arg("---test")
        .output()
        .expect("failed to execute");
    // Should not fail with a clap error (exit code 2), but rather report invalid name
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid") || output.status.code() == Some(1),
        "should handle hyphen-valued names: stdout={stdout}"
    );
}
