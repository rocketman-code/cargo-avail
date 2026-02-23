use std::process::Command;

fn cargo_avail() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cargo-avail"))
}

#[test]
fn json_flag_outputs_ndjson() {
    let output = cargo_avail()
        .args(["--json", "std"])
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("should be valid JSON");
    assert_eq!(parsed["name"], "std");
    assert_eq!(parsed["status"], "reserved");
}

#[test]
fn json_flag_error_includes_error_field() {
    let output = cargo_avail()
        .args(["--json", "foo+bar"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("should be valid JSON");
    assert_eq!(parsed["name"], "foo+bar");
    assert_eq!(parsed["status"], "invalid");
    assert!(parsed["error"].is_string(), "should have error field");
}

#[test]
fn json_flag_multiple_names_outputs_ndjson_lines() {
    let output = cargo_avail()
        .args(["--json", "std", "core"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2, "should have 2 NDJSON lines: {stdout}");
    for line in &lines {
        let _: serde_json::Value =
            serde_json::from_str(line).expect("each line should be valid JSON");
    }
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
fn version_flag_prints_version() {
    let output = cargo_avail()
        .arg("--version")
        .output()
        .expect("failed to execute");
    assert!(output.status.success(), "should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cargo-avail"),
        "should contain crate name: {stdout}"
    );
}

#[test]
fn cargo_subcommand_strips_avail() {
    // When cargo invokes `cargo-avail avail <args>`, the "avail" should be stripped.
    // Simulate cargo's invocation by setting the CARGO env var, which cargo sets
    // for external subcommands.
    let output = cargo_avail()
        .env("CARGO", "cargo")
        .args(["avail", "std"])
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should only have one result (std), not two (avail + std)
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "should strip 'avail' subcommand: {stdout}");
    assert!(
        stdout.contains("reserved"),
        "should show std as reserved: {stdout}"
    );
}

#[test]
fn cargo_subcommand_flags_work() {
    // Flags should work when invoked via cargo subcommand pattern.
    // Simulate cargo's invocation by setting the CARGO env var.
    let output = cargo_avail()
        .env("CARGO", "cargo")
        .args(["avail", "--version"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success(), "should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cargo-avail"),
        "should print version: {stdout}"
    );
}

#[test]
fn direct_invocation_treats_avail_as_crate_name() {
    // When invoked directly (not via `cargo avail`), "avail" should be treated as
    // a crate name to check, not stripped as a subcommand prefix.
    // The CARGO env var is NOT set here, simulating direct invocation.
    let output = cargo_avail()
        .env_remove("CARGO")
        .args(["avail", "std"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    // Should have two results: one for "avail" and one for "std"
    assert_eq!(
        lines.len(),
        2,
        "should treat 'avail' as a crate name in direct invocation: {stdout}"
    );
    assert!(
        stdout.contains("avail"),
        "should check the crate name 'avail': {stdout}"
    );
    assert!(stdout.contains("std"), "should also check 'std': {stdout}");
}

#[test]
fn flags_work_after_positional_args() {
    // --json after positional names should be recognized as a flag, not a crate name
    let output = cargo_avail()
        .args(["std", "--json"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("should be valid JSON");
    assert_eq!(parsed["name"], "std");
    assert_eq!(parsed["status"], "reserved");
}

#[test]
fn hyphen_valued_names_via_separator() {
    // Names starting with hyphens can be passed after `--` to prevent clap
    // from treating them as flags
    let output = cargo_avail()
        .args(["--", "---test"])
        .output()
        .expect("failed to execute");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid"),
        "should report invalid name: stdout={stdout}"
    );
}
