use std::process::Command;

fn latchix() -> Command {
    Command::new(env!("CARGO_BIN_EXE_latchix"))
}

#[test]
fn version_is_available() {
    let output = latchix().arg("--version").output().expect("run latchix");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("latchix 0.1.0"));
}

#[test]
fn help_is_available() {
    let output = latchix().arg("--help").output().expect("run latchix");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("doctor"));
}

#[test]
fn stubs_exit_with_not_implemented() {
    for command in ["migrate", "backup", "bootstrap-token"] {
        let mut invocation = latchix();
        invocation.arg(command);
        if command == "backup" {
            invocation.args(["--output", "/backups/latchix.tar"]);
        }
        let output = invocation.output().expect("run latchix");
        assert_eq!(output.status.code(), Some(3), "{command} should exit 3");
    }
}

#[test]
fn missing_config_exits_with_config_error() {
    let output = latchix()
        .args(["--config", "/nonexistent/latchix/config.toml", "health"])
        .output()
        .expect("run latchix");
    assert_eq!(output.status.code(), Some(4));
}
