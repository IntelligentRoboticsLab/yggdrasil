//! Construct version in the `commit-hash date channel` format
//! Based on dioxus-cli versioning scheme

use std::{env, path::PathBuf, process::Command};

fn main() {
    set_rerun_opts();
    set_commit_info();
}

fn set_rerun_opts() {
    let mut manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("`CARGO_MANIFEST_DIR` is always set by cargo."),
    );

    while manifest_dir.parent().is_some() {
        let head_ref = manifest_dir.join(".git/HEAD");
        if head_ref.exists() {
            println!("cargo:rerun-if-changed={}", head_ref.display());
            return;
        }

        manifest_dir.pop();
    }

    println!("cargo:warning=Could not find `.git/HEAD` from manifest dir!");
}

fn set_commit_info() {
    // This command is executed in "crates/sindri"", and as such
    // we need to pass "." as last argument, to tell git that we
    // only care about changes in that directory.
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg(".")
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            println!("cargo:warning=Non-zero process exit while obtaining commit hash for sindri!");
            return;
        }
        Err(e) => {
            println!("cargo:warning=Failed to spawn git process: {}", e);
            return;
        }
    };

    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace();
    let mut next = || parts.next().unwrap();

    println!("cargo:rustc-env=SINDRI_COMMIT_HASH={}", next());
    println!("cargo:rustc-env=SINDRI_COMMIT_SHORT_HASH={}", next());
    println!("cargo:rustc-env=SINDRI_COMMIT_DATE={}", next())
}
