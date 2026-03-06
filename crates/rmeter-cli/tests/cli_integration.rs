use std::process::Command;

fn cli_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rmeter-cli"))
}

#[test]
fn help_flag_shows_usage() {
    let output = cli_binary().arg("--help").output().expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rmeter-cli"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("validate"));
}

#[test]
fn validate_nonexistent_file_fails() {
    let output = cli_binary()
        .args(["validate", "/tmp/nonexistent-plan-file.rmeter"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid plan") || stderr.contains("Error") || stderr.contains("No such file"));
}

#[test]
fn validate_valid_plan_file() {
    // Create a minimal valid plan file
    let plan_json = r#"{
        "id": "00000000-0000-0000-0000-000000000001",
        "name": "CLI Test Plan",
        "description": "test",
        "thread_groups": [{
            "id": "00000000-0000-0000-0000-000000000002",
            "name": "TG1",
            "num_threads": 1,
            "ramp_up_seconds": 0,
            "loop_count": {"type": "finite", "count": 1},
            "requests": [],
            "enabled": true
        }],
        "variables": [],
        "csv_data_sources": [],
        "format_version": 1
    }"#;

    let tmp = std::env::temp_dir().join("rmeter_cli_test_valid.rmeter");
    std::fs::write(&tmp, plan_json).unwrap();

    let output = cli_binary()
        .args(["validate", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run");

    std::fs::remove_file(&tmp).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CLI Test Plan"));
    assert!(stdout.contains("valid"));
}

#[test]
fn validate_invalid_json_fails() {
    let tmp = std::env::temp_dir().join("rmeter_cli_test_invalid.rmeter");
    std::fs::write(&tmp, "not valid json").unwrap();

    let output = cli_binary()
        .args(["validate", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run");

    std::fs::remove_file(&tmp).ok();

    assert!(!output.status.success());
}

#[test]
fn run_nonexistent_file_fails() {
    let output = cli_binary()
        .args(["run", "/tmp/nonexistent-plan-file.rmeter"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
}
