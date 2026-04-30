use std::process::Command;

#[test]
fn help_lists_build_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_video-gen"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("build"));
}

#[test]
fn build_help_lists_existing_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_video-gen"))
        .args(["build", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--input-dir"));
    assert!(stdout.contains("--lead-in"));
    assert!(stdout.contains("--audio-bitrate"));
}

#[test]
fn invalid_duration_exits_with_user_error_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_video-gen"))
        .args(["build", "--lead-in=-1"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--lead-in"));
    assert!(stderr.contains("non-negative"));
}
