use std::{
    ffi::OsStr,
    fmt::{Debug, Write},
    process::Stdio,
    string::FromUtf8Error,
    time::Duration,
};

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use miette::{Context, Diagnostic, Result};
use regex::Regex;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[derive(Error, Diagnostic, Debug)]
enum CargoErrorKind {
    #[error(transparent)]
    #[diagnostic(help("Failed to spawn cargo child process!"))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to deserialize cargo unit-graph!"))]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to parse regex pattern!"))]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to parse stderr"))]
    FromUtf8(#[from] FromUtf8Error),

    #[error(
        "Cargo execution failed:
        {0}"
    )]
    #[diagnostic(help("Cargo output is printed above!"))]
    Cargo(String),
}
async fn cargo<I, S>(args: I) -> Result<(), CargoErrorKind>
where
    I: IntoIterator<Item = S> + Debug + Clone,
    S: AsRef<OsStr>,
{
    let progress_regex = Regex::new(r"\]\s+(\d+)/(\d+)")?;
    let module_regex = Regex::new(r"\d+\d+: (.+?)(?:\r\n|\r|\n|$)")?;
    let m = MultiProgress::new();
    let pb = m.add(ProgressBar::new(1000));
    pb.set_style(
        ProgressStyle::with_template("{prefix} 🔨 Building yggdrasil... ({elapsed})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .tick_strings(&[".  ", ".. ", "...", "   "])
            .progress_chars("#>-"),
    );
    pb.set_prefix(style("[1/1]").bold().dim().to_string());

    // let bar = m.add(ProgressBar::new(100 as u64));
    // bar.set_style(ProgressStyle::default_bar().progress_chars("█▛▌▖  "));

    let spinner_style = ProgressStyle::with_template("{prefix:.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let mut output = Command::new("cargo")
        .env("CARGO_TERM_PROGRESS_WHEN", "always")
        .env("CARGO_TERM_PROGRESS_WIDTH", "250")
        .args(args)
        .args(["--color", "always"]) // always pass color, cargo doesn't pass color when it detects it's piped
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stderr = "".to_string();
    if let Some(stdout) = output.stderr.take() {
        let reader = BufReader::new(stdout).lines();

        // Process each line asynchronously
        tokio::pin!(reader);

        let mut module_bars: Vec<ProgressBar> = Vec::new();
        while let Some(line) = reader.next_line().await? {
            stderr.push_str(&line.clone());
            // println!("{line}");
            if let Some(captures) = progress_regex.captures(&line) {
                let current = captures[1].parse::<u64>().unwrap();
                let total = captures[2].parse::<u64>().unwrap();

                pb.set_length(total);
                pb.set_position(current);
                // bar.set_length(total);
                // bar.set_position(current);

                if let Some(modules_captures) = module_regex.captures(&line) {
                    let modules = modules_captures[1]
                        .split(", ")
                        .map(|it| it.trim().to_string())
                        .collect::<Vec<_>>();
                    let len = if module_bars.len() > modules.len() {
                        module_bars.len()
                    } else {
                        modules.len()
                    };
                    for i in 0..len {
                        if module_bars.len() <= i {
                            let mbar = m.add(ProgressBar::new(total));
                            mbar.set_style(spinner_style.clone());
                            mbar.set_prefix(format!("[{}/{}]", i + 1, modules.len()));
                            mbar.set_message(modules[i].clone());
                            mbar.enable_steady_tick(Duration::from_millis(120));
                            module_bars.push(mbar);
                            continue;
                        }

                        let mbar = &module_bars[i];
                        if modules.len() <= i && !mbar.is_finished() {
                            mbar.finish_and_clear();
                        } else if modules.len() > i {
                            module_bars[i].set_message(modules[i].clone());
                            module_bars[i].inc(1);
                        }
                    }
                    // bar.println(format!("Modules: {:?}",modules));
                }
            }
        }
    }

    if !output.wait().await?.success() {
        // let stderr = String::).map_err(CargoErrorKind::FromUtf8)?;
        eprintln!("{stderr}");
        m.clear()?;
        return Err(CargoErrorKind::Cargo("Failed".to_string()))?;
    }

    pb.finish_with_message("Built successfully!");
    // bar.finish_and_clear();

    m.clear()?;
    Ok(())
}

pub async fn build(binary: &str, release: bool, target: Option<&str>) -> Result<()> {
    let mut cargo_args = vec!["build", "-p"];
    cargo_args.push(binary);

    if release {
        cargo_args.push("--release");
    }

    if let Some(target) = target.as_ref() {
        cargo_args.push("--target");
        cargo_args.push(target);
    }

    cargo(cargo_args)
        .await
        .wrap_err("Failed to build yggdrasil!")
}
