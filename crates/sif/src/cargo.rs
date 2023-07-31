use std::{ffi::OsStr, fmt::Debug, process::Stdio};

use miette::{IntoDiagnostic, Result};
use spinoff::{spinners, Color, Spinner};
use tokio::process::Command;

pub async fn cargo<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr>,
{
    println!("spawning cargo with args: {:?}", args);
    let spinner = Spinner::new(spinners::Dots, "Building...", Color::Green);
    let output = Command::new("cargo")
        .args(args)
        .arg("--color") // always pass color, cargo doesn't pass color when it detects it's piped
        .arg("always")
        .stderr(Stdio::null())
        .output()
        .await
        .into_diagnostic()?;

    if !output.status.success() {
        spinner.fail("Failed to build yggdrasil!");
        println!("{}", String::from_utf8(output.stderr).into_diagnostic()?);
        return Ok(());
    }

    spinner.success("Finished building yggdrasil!");

    Ok(())
}
