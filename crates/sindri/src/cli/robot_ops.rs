use clap::{builder::ArgPredicate, Parser};
use colored::Colorize;
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use miette::{miette, Context, IntoDiagnostic};
use ssh2::{ErrorCode, OpenFlags, OpenType, Session, Sftp};
use std::{
    borrow::Cow,
    fs,
    io::BufWriter,
    net::Ipv4Addr,
    path::{Component, Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use tokio::{self, net::TcpStream};
use walkdir::{DirEntry, WalkDir};

use crate::{
    cargo::{self, find_bin_manifest, Profile},
    config::{Robot, SindriConfig},
    error::{Error, Result},
};

const ROBOT_TARGET: &str = "x86_64-unknown-linux-gnu";
const RELEASE_PATH_REMOTE: &str = "./target/x86_64-unknown-linux-gnu/release/yggdrasil";
const RELEASE_PATH_LOCAL: &str = "./target/release/yggdrasil";
const DEPLOY_PATH: &str = "./deploy/yggdrasil";
const CONNECTION_TIMEOUT: u64 = 5;
const LOCAL_ROBOT_ID_STR: &str = "0";

/// The size of the `BufWriter`'s buffer.
///
/// This is currently set to 1 MiB, as the [`Write`] implementation for [`ssh2::sftp::File`]
/// is rather slow due to the locking mechanism.
const UPLOAD_BUFFER_SIZE: usize = 1024 * 1024;

/// Because clap does not support HashMaps, we have to implement a vector with
/// a wrapper.
#[derive(Clone, Debug)]
pub struct RobotEntry {
    pub robot_number: u8,
    pub player_number: Option<u8>,
}

impl FromStr for RobotEntry {
    type Err = miette::Report;

    // Parses robot:player_number pairs. Player numbers are optional, if they
    // are not passed, defaults are used. Valid arguments pairs could be: "23:1"
    // or "24".
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut m = s.split(':');
        let robot: u8 = m.next().unwrap().parse().into_diagnostic()?;
        let player_number: Option<u8> = m
            .next()
            .map(|val| val.parse())
            .transpose()
            .into_diagnostic()?;

        Ok(RobotEntry {
            robot_number: robot,
            player_number,
        })
    }
}

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsRobotOps {
    /// Team number [default: Set in `sindri.toml`]
    #[clap(short, long)]
    pub team: Option<u8>,

    /// Whether to embed the rerun viewer for debugging [default: false]
    #[clap(long, short)]
    pub rerun: bool,

    /// For running Yggdrasil locally with fake-lola
    #[clap(long, short)]
    pub local: bool,

    /// Optional argument that can be passed to make robots switch networks
    #[clap(long, short, required = false)]
    pub network: Option<String>,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long, default_value_ifs([("network", ArgPredicate::IsPresent, "true")]))]
    pub wired: bool,

    /// Specify bin target
    #[clap(global = true, long, default_value = "yggdrasil")]
    pub bin: String,

    /// Whether to use alsa
    #[clap(
        long,
        default_value_ifs([
            ("local", "true", Some("true")),
            ("bin", "yggdrasil", Some("false")),
        ]),
    )]
    pub no_alsa: bool,

    /// Whether the command prints all progress
    #[clap(long, short)]
    pub silent: bool,

    /// Number of the robot to deploy to.
    #[clap(
        required(false),
        required_unless_present("local"),
        default_value_if("local", "true", Some(LOCAL_ROBOT_ID_STR)),
        conflicts_with("local"),
        value_parser = clap::value_parser!(RobotEntry),
    )]
    pub robots: Vec<RobotEntry>,

    #[clap(long, default_value = "0.1")]
    pub volume: f64,

    /// Whether to use the `timings` feature in the `yggdrasil` binary [default: false]
    ///
    /// This feature is used to log spans to the [Tracy] profiler, which can be
    /// used to visualize the execution time of systems in yggdrasil.
    ///
    /// Sindri will attempt to start the profiler, and expects `Tracy` to be installed.
    ///
    /// By default, the profiler will attempt to connect using the robot's ip address,
    /// and port `8086`. This can be changed by setting the `TRACY_CLIENT` environment variable.
    ///
    /// [Tracy]: https://github.com/wolfpld/tracy
    #[clap(long)]
    pub timings: bool,
}

impl ConfigOptsRobotOps {
    pub fn robots(&self) -> Vec<RobotEntry> {
        self.robots.clone()
    }

    /// Get a specific robot
    pub fn get_robot(&self, robot: u8, config: &SindriConfig) -> miette::Result<Robot> {
        config.robot(robot, self.wired).ok_or(miette!(format!(
            "Invalid robot specified, number {} is not configured!",
            robot
        )))
    }

    /// Get robot information for a single robot, when there is just a single robot
    pub fn get_first_robot(&self, config: &SindriConfig) -> miette::Result<Robot> {
        if self.robots.is_empty() {
            return Err(miette!("Pass at least one robot number as argument"));
        }

        self.get_robot(self.robots[0].robot_number, config)
    }
}

/// Enum used to determine the type of progress bar to use
#[derive(Clone)]
pub enum Output {
    Silent,
    Single(ProgressBar),
    Multi(ProgressBar),
}

impl Output {
    pub fn should_print(&self) -> bool {
        matches!(self, Output::Single(_) | Output::Multi(_))
    }

    pub fn set_message(&self, msg: impl Into<Cow<'static, str>>) {
        match self {
            Output::Silent => {}
            Output::Single(pb) | Output::Multi(pb) => {
                pb.set_message(msg.into());
            }
        }
    }

    pub fn compile_phase(&self) {
        match self {
            Output::Silent => {}
            Output::Single(pb) | Output::Multi(pb) => {
                pb.set_message(format!(
                    "{} {} {}",
                    "Compiling".bright_blue().bold(),
                    "yggdrasil".bold(),
                    "(release: ".dimmed(),
                ));
            }
        }
    }

    pub fn connecting_phase(&self, addr: &Ipv4Addr) {
        match self {
            Output::Silent => {}
            Output::Single(pb) | Output::Multi(pb) => {
                pb.set_message(format!(
                    "{} {} {}",
                    "Connecting".bright_blue().bold(),
                    "to".dimmed(),
                    addr.to_string().clone().bold(),
                ));
            }
        }
    }

    pub fn upload_phase(&self, num_files: u64) {
        match self {
            Output::Silent => {}
            Output::Single(pb) => {
                pb.set_message(format!("{}", "Ensuring host directories exist".dimmed()));
                pb.set_prefix(format!("{}", "Uploading".blue().bold()));
                pb.set_style(
                    ProgressStyle::with_template(
                        "   {prefix:.blue.bold} {msg} [{bar:.blue/cyan}] {spinner:.blue.bold}",
                    )
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .progress_chars("=>-"),
                );
            }
            Output::Multi(pb) => {
                pb.set_length(num_files);
                pb.set_style(
                    ProgressStyle::with_template(
                        "   {prefix:.blue.bold} [{bar:.blue/cyan}]: {msg}",
                    )
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .progress_chars("=>-"),
                );
                pb.set_prefix(format!("{}", "Uploading".blue().bold()));
            }
        }
    }

    pub fn finished_deploying(&self, ip: &Ipv4Addr) {
        match self {
            Output::Silent => {}
            Output::Single(pb) | Output::Multi(pb) => {
                pb.set_style(
                    ProgressStyle::with_template("    {prefix:.blue.bold} to {msg}").unwrap(),
                );
                pb.set_prefix(format!("{}", "Deployed".blue().bold()));
                pb.set_message(ip.to_string());
                pb.finish();
            }
        }
    }

    pub fn spinner(&self) {
        match self {
            Output::Silent => {}
            Output::Single(pb) | Output::Multi(pb) => {
                pb.reset();
                pb.enable_steady_tick(Duration::from_millis(80));
                pb.set_style(
                    ProgressStyle::with_template("{prefix:.blue.bold} {msg} {spinner:.blue.bold}")
                        .unwrap()
                        .progress_chars("=>-")
                        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
                );
                pb.set_prefix("")
            }
        }
    }
}

/// Environment variables that are required to cross compile for the robot, depending
/// on the current host architecture.
mod cross {
    #[cfg(target_os = "linux")]
    pub const ENV_VARS: &[(&str, &str)] = &[];

    #[cfg(target_os = "macos")]
    pub const ENV_VARS: &[(&str, &str)] = &[
        (
            "PKG_CONFIG_PATH",
            // homebrew directory is different for x86_64 and aarch64 macs!
            #[cfg(target_arch = "aarch64")]
            "/opt/homebrew/opt/x86_64-unknown-linux-gnu-alsa-lib/lib/x86_64-unknown-linux-gnu/pkgconfig",
            #[cfg(target_arch = "x86_64")]
            "/usr/local/opt/x86_64-unknown-linux-gnu-alsa-lib/lib/x86_64-unknown-linux-gnu/pkgconfig",
        ),
        ("PKG_CONFIG_ALLOW_CROSS", "1"),
        ("TARGET_CC", "x86_64-unknown-linux-gnu-gcc"),
        ("TARGET_CXX", "x86_64-unknown-linux-gnu-g++"),
        ("TARGET_AR", "x86_64-unknown-linux-gnu-ar"),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "x86_64-unknown-linux-gnu-gcc",
        ),
        (
            // This is required for the `tracy-client-sys` crate to cross-compile on mac
            // https://github.com/wolfpld/tracy/issues/730
            "TRACY_CLIENT_SYS_CXXFLAGS",
            "-D__STDC_FORMAT_MACROS=1"
        )
    ];
}

/// Modify the default network for a specific robot
pub(crate) async fn change_single_network(
    robot: &Robot,
    network: String,
    output: Output,
) -> Result<()> {
    match &output {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.set_prefix("    Changing");
            pb.set_message(format!(
                "{} {}",
                "network to".bold(),
                network.bright_yellow()
            ));
        }
        Output::Single(pb) => {
            pb.set_prefix("    Changing");
            pb.set_message(format!(
                "{} {}",
                "network to".bold(),
                network.bright_yellow()
            ));
        }
    }

    robot
        .ssh::<&str, &str>(format!("echo {} > /etc/network_config", network), [], true)?
        .wait()
        .await?;

    robot
        .ssh::<&str, &str>("sudo systemctl restart network_config.service", [], true)?
        .wait()
        .await?;

    match output {
        Output::Silent => {}
        Output::Multi(pb) => pb.println(format!(
            "     {} {} {}",
            "Changed".bold().blue(),
            "network to".bold(),
            network.bright_yellow()
        )),
        Output::Single(pb) => pb.println(format!(
            "     {} {} {}",
            "Changed".bold().blue(),
            "network to".bold(),
            network.bright_yellow()
        )),
    }

    Ok(())
}

/// Compile yggdrasil
pub(crate) async fn compile(config: ConfigOptsRobotOps, output: Output) -> miette::Result<()> {
    find_bin_manifest(&config.bin)
        .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

    let mut features = vec![];
    if !config.no_alsa {
        features.push("alsa");
    }
    if config.rerun {
        features.push("rerun");
    }
    if config.local {
        features.push("local");
    }
    if config.timings {
        features.push("timings");
    }

    let target = if config.local {
        None
    } else {
        Some(ROBOT_TARGET)
    };

    let pb = match output.clone() {
        Output::Silent => {
            let pb = ProgressBar::new_spinner();
            pb.set_draw_target(ProgressDrawTarget::hidden());
            pb
        }
        Output::Single(pb) | Output::Multi(pb) => pb,
    };

    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::with_template(
            "   {prefix:.green.bold} yggdrasil {msg} {spinner:.green.bold}",
        )
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    pb.set_message(format!(
        "{}{}, {}{}{}",
        "(release: ".dimmed(),
        "true".red(),
        "target: ".dimmed(),
        ROBOT_TARGET.bold(),
        ")".dimmed()
    ));
    pb.set_prefix("Compiling");

    cargo::build(
        &config.bin,
        Profile::Release,
        target,
        &features,
        Some(cross::ENV_VARS.to_vec()),
    )
    .await?;

    if output.should_print() {
        pb.println(format!(
            "{} {} {}{}, {}{}{}",
            "   Compiling".green().bold(),
            "yggdrasil".bold(),
            "(release: ".dimmed(),
            "true".red(),
            "target: ".dimmed(),
            ROBOT_TARGET.bold(),
            ")".dimmed()
        ));
        pb.println(format!(
            "{} in {}",
            "    Finished".green().bold(),
            HumanDuration(pb.elapsed()),
        ));
        pb.reset_elapsed();
    }

    let release_path = if config.local {
        RELEASE_PATH_LOCAL
    } else {
        RELEASE_PATH_REMOTE
    };

    // Copy over the files that need to be deployed
    fs::copy(release_path, DEPLOY_PATH)
        .into_diagnostic()
        .wrap_err("Failed to copy binary to deploy directory!")?;

    Ok(())
}

/// Start the yggdrasil service on a specific robot
pub(crate) async fn start_single_yggdrasil_service(robot: &Robot, output: Output) -> Result<()> {
    match &output {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.set_message(format!(
                "    {} {} {}",
                "Starting".bright_green().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
        Output::Single(pb) => {
            pb.set_message(format!(
                "    {} {} {}",
                "Starting".bright_green().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
    }

    robot
        .ssh::<&str, &str>("sudo systemctl restart yggdrasil", [], true)?
        .wait()
        .await?;

    match output {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.println(format!(
                "     {} {} {}",
                "Started".bright_green().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
        Output::Single(pb) => {
            pb.println(format!(
                "     {} {} {}",
                "Started".bright_green().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
    }

    Ok(())
}

/// Stop the yggdrasil service on a specific robot
pub(crate) async fn stop_single_yggdrasil_service(robot: &Robot, output: Output) -> Result<()> {
    match &output {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.set_message(format!(
                "   {} {} {}",
                "Stopping".bright_red().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
        Output::Single(pb) => {
            pb.set_message(format!(
                "   {} {} {}",
                "Stopping".bright_red().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
    }

    robot
        .ssh::<&str, &str>("sudo systemctl stop yggdrasil", [], true)?
        .wait()
        .await?;

    match output {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.println(format!(
                "     {} {} {}",
                "Stopped".bright_red().bold(),
                "yggdrasil service on".dimmed(),
                robot.ip()
            ));
        }
        Output::Single(pb) => {
            pb.println(format!(
                "     {} {} {}",
                "Stopped".bright_red().bold(),
                "yggdrasil service on",
                robot.ip()
            ));
        }
    }

    Ok(())
}

/// Copy the contents of the 'deploy' folder to the robot.
pub(crate) async fn upload_to_robot(addr: &Ipv4Addr, output: Output) -> Result<()> {
    output.connecting_phase(addr);
    let sftp = create_sftp_connection(addr).await?;
    match output.clone() {
        Output::Silent => {}
        Output::Multi(pb) => {
            pb.set_message(format!("{}", "Connected".bright_blue().bold()));
        }
        Output::Single(pb) => {
            pb.set_message(format!("{}", "  Connected".bright_blue().bold()));
        }
    }

    let entries: Vec<DirEntry> = WalkDir::new("./deploy")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .collect();
    let num_files = entries
        .iter()
        .filter(|e| e.metadata().unwrap().is_file())
        .count();

    output.upload_phase(num_files as u64);

    for entry in entries.iter() {
        let remote_path = get_remote_path(entry.path());

        if entry.path().is_dir() {
            // Ensure all directories exist on remote
            ensure_directory_exists(&sftp, remote_path)?;
            continue;
        }

        let file_remote = sftp
            .open_mode(
                &remote_path,
                OpenFlags::WRITE | OpenFlags::TRUNCATE,
                0o777,
                OpenType::File,
            )
            .map_err(|e| Error::Sftp {
                source: e,
                msg: format!("Failed to open remote file {:?}!", entry.path()),
            })?;

        let mut file_local = std::fs::File::open(entry.path())?;

        match output.clone() {
            Output::Silent => {}
            Output::Multi(pb) => {
                pb.set_message(format!("{}", entry.path().to_string_lossy().dimmed()));
            }
            Output::Single(pb) => {
                pb.set_length(file_local.metadata()?.len());
                pb.set_message(format!("{}", entry.path().to_string_lossy()));
            }
        }

        // Since `file_remote` impl's Write, we can just copy directly using a BufWriter!
        // The Write impl is rather slow, so we set a large buffer size of 1 mb.
        let mut buf_writer = BufWriter::with_capacity(UPLOAD_BUFFER_SIZE, file_remote);

        match output.clone() {
            Output::Silent => {
                std::io::copy(&mut file_local, &mut buf_writer)?;
            }
            Output::Multi(pb) => {
                std::io::copy(&mut file_local, &mut buf_writer)?;
                pb.inc(1);
            }
            Output::Single(pb) => {
                std::io::copy(&mut file_local, &mut pb.wrap_write(buf_writer))
                    .map_err(Error::Io)?;

                pb.println(format!(
                    "{} {}",
                    "    Uploaded".bright_blue().bold(),
                    entry.path().to_string_lossy().dimmed()
                ));
            }
        }
    }

    output.spinner();

    if let Output::Multi(pb) = &output {
        pb.set_message(format!(
            "    {} {}",
            "Uploaded".green().bold(),
            addr.to_string().red()
        ));
    }

    Ok(())
}

async fn create_sftp_connection(ip: &Ipv4Addr) -> Result<Sftp> {
    let tcp = tokio::time::timeout(
        Duration::from_secs(CONNECTION_TIMEOUT),
        TcpStream::connect(format!("{ip}:22")),
    )
    .await
    .map_err(Error::Elapsed)??;
    let mut session = Session::new().map_err(|e| Error::Sftp {
        source: e,
        msg: "Failed to create ssh session!".to_owned(),
    })?;

    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| Error::Sftp {
        source: e,
        msg: "Failed to perform ssh handshake!".to_owned(),
    })?;
    session
        .userauth_password("nao", "")
        .map_err(|e| Error::Sftp {
            source: e,
            msg: "Failed to authenticate using ssh!".to_owned(),
        })?;

    session.sftp().map_err(|e| Error::Sftp {
        source: e,
        msg: "Failed to create sftp session!".to_owned(),
    })
}

fn ensure_directory_exists(sftp: &Sftp, remote_path: impl AsRef<Path>) -> Result<()> {
    match sftp.mkdir(remote_path.as_ref(), 0o777) {
        Ok(()) => Ok(()),
        // Error code 4, means the directory already exists, so we can ignore it
        Err(error) if error.code() == ErrorCode::SFTP(4) => Ok(()),
        Err(error) => Err(Error::Sftp {
            source: error,
            msg: "Failed to ensure directory exists".to_owned(),
        }),
    }
}

fn get_remote_path(local_path: &Path) -> PathBuf {
    let mut remote_path = PathBuf::from("/home/nao");

    for component in local_path.components() {
        // Would be nice to replace this with an if let chain once https://github.com/rust-lang/rust/issues/53667#issuecomment-1374336460 is stable.
        match component {
            // Prevent "deploy" from being added to the remote path, as we'll deploy directly to home directory.
            Component::Normal(c) if c != "deploy" => remote_path.push(c),
            // Any other component kind should ignored, such as ".".
            _ => continue,
        }
    }

    remote_path
}
