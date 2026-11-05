#![forbid(unsafe_code)]

use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let command = env::args().nth(1).unwrap_or_else(|| "help".into());
    let ok = match command.as_str() {
        "check" => check(),
        "audit" => audit(),
        "build" => build(),
        "image" => image(),
        "ci" => check() && audit() && build() && image(),
        "help" | "--help" | "-h" => {
            print_help();
            true
        }
        other => {
            eprintln!("unknown task: {other}");
            print_help();
            false
        }
    };

    ExitCode::from(u8::from(!ok))
}

fn check() -> bool {
    run("cargo", &["fmt", "--all", "--", "--check"])
        && run(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )
        && run("cargo", &["test", "--workspace", "--all-features"])
        && run(
            "cargo",
            &["doc", "--workspace", "--no-deps", "--all-features"],
        )
}

fn audit() -> bool {
    run(&tool("cargo-deny"), &["check"]) && run(&tool("cargo-audit"), &["audit"])
}

fn build() -> bool {
    build_musl()
        && run(
            "cargo",
            &["build", "--locked", "--release", "-p", "latchix-cli"],
        )
}

fn build_musl() -> bool {
    if command_exists("x86_64-linux-musl-gcc") {
        return run(
            "cargo",
            &[
                "build",
                "--locked",
                "--release",
                "--target",
                "x86_64-unknown-linux-musl",
                "-p",
                "latchix-cli",
            ],
        );
    }

    if !command_exists("podman") {
        eprintln!(
            "x86_64-linux-musl-gcc is missing and podman is not available. \
             Install musl-gcc or podman."
        );
        return false;
    }

    let source = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("cannot read the current directory: {error}");
            return false;
        }
    };
    let mount = format!("{}:/src:Z", source.display());
    let script = "apk add --no-cache cmake gcc make musl-dev nasm perl && \
                  cargo build --locked --release --target x86_64-unknown-linux-musl -p latchix-cli";

    println!("+ podman run --rm -v {mount} -w /src rust:1.99.1-alpine");
    match Command::new("podman")
        .args([
            "run",
            "--rm",
            "-v",
            &mount,
            "-w",
            "/src",
            "docker.io/library/rust:1.99.1-alpine",
            "sh",
            "-c",
            script,
        ])
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("container build failed with {status}");
            false
        }
        Err(error) => {
            eprintln!("cannot run podman: {error}");
            false
        }
    }
}

fn image() -> bool {
    run(
        "podman",
        &["build", "--format", "docker", "-t", "latchix:dev", "."],
    )
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn tool(name: &str) -> String {
    let local = format!(".tools/bin/{name}");
    if Path::new(&local).exists() {
        local
    } else {
        name.into()
    }
}

fn run(program: &str, args: &[&str]) -> bool {
    println!("+ {program} {}", args.join(" "));
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("command failed with {status}");
            false
        }
        Err(error) => {
            eprintln!("cannot run {program}: {error}");
            false
        }
    }
}

fn print_help() {
    println!("Usage: cargo xtask <task>");
    println!();
    println!("Tasks:");
    println!("  check   format, clippy, test, and doc");
    println!("  audit   cargo-deny and cargo-audit");
    println!("  build   musl and glibc release binaries");
    println!("  image   build the OCI image with Podman");
    println!("  ci      check, audit, build, and image");
    println!("  help    this message");
}
