use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use miette::{miette, Context, IntoDiagnostic};
use ssh2::{ErrorCode, OpenFlags, OpenType, Session, Sftp};
use std::{
    fs::{self},
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

const BINARY_NAME: &str = "yggdrasil";
const ROBOT_TARGET: &str = "x86_64-unknown-linux-gnu";
const RELEASE_PATH_REMOTE: &str = "./target/x86_64-unknown-linux-gnu/release/yggdrasil";
const RELEASE_PATH_LOCAL: &str = "./target/release/yggdrasil";
const DEPLOY_PATH: &str = "./deploy/yggdrasil";
const CONNECTION_TIMEOUT: u64 = 5;
const LOCAL_ROBOT_ID_STR: &str = "0";
const DEFAULT_NETWORK: &str = "DNT_5G";

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

/// Trait used to implement fuctionality on `[Vec<RobotEntry>]`
trait RobExt {
    /// Function that retrieves all robot numbers
    fn robot_numbers(&self) -> Vec<u8>;
}

impl RobExt for Vec<RobotEntry> {
    fn robot_numbers(&self) -> Vec<u8> {
        self.iter()
            .map(
                |RobotEntry {
                     robot_number: robot,
                     ..
                 }| *robot,
            )
            .collect()
    }
}

impl FromStr for RobotEntry {
    type Err = miette::Report;

    // Parses robot:player_number pairs. Player numbers are optional, if they are not passed, defaults are used. Valid arguments pairs could be: "23:1" or "24".
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
    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long)]
    pub wired: bool,

    /// Team number [default: Set in `sindri.toml`]
    #[clap(short, long)]
    pub team_number: Option<u8>,

    /// Whether to embed the rerun viewer for debugging [default: false]
    #[clap(long, short)]
    pub rerun: bool,

    #[clap(long, short)]
    pub local: bool,

    /// Specify bin target
    #[clap(global = true, long, default_value = "yggdrasil")]
    pub bin: String,

    /// Whether to use alsa
    #[clap(
        long,
        short,
        default_value_ifs([
            ("local", "true", Some("false")),
            ("bin", "yggdrasil", Some("true")),
        ]),
    )]
    pub alsa: bool,

    /// Whether the command prints all progress
    #[clap(long, short)]
    pub silent: bool,

    #[clap(long, short, required=false, default_value=DEFAULT_NETWORK)]
    pub network: String,

    /// Number of the robot to deploy to.
    #[clap(
        required(false),
        required_unless_present("local"),
        default_value_if("local", "true", Some(LOCAL_ROBOT_ID_STR)),
        conflicts_with("local"),
        value_parser = clap::value_parser!(RobotEntry),
    )]
    pub robots: Vec<RobotEntry>,
}

/// Abstraction containing functionality useful for deploying code
pub struct RobotOps {
    pub sindri_config: SindriConfig,
    pub config: ConfigOptsRobotOps,
}

/// Used to indicate whether actions should be verbose or not
#[derive(Clone)]
pub enum SindriProgressBar {
    None,
    Single(ProgressBar),
    Multi(ProgressBar),
}

#[derive(Clone, Copy)]
pub enum Output {
    Silent,
    Verbose,
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
    ];
}

impl RobotOps {
    pub async fn change_networks(&self, network: String) -> miette::Result<()> {
        let mut threads: Vec<_> = Vec::new();

        for robot in self.config.robots.robot_numbers() {
            let robot = self.get_robot(robot)?;
            let n = network.clone();
            let thread = tokio::spawn(async move {
                change_single_network(robot, n).await.unwrap();
            });
            threads.push(thread);
        }

        for temp_thread in threads {
            temp_thread.await.into_diagnostic()?;
        }

        Ok(())
    }

    /// Compile yggdrasil
    pub async fn compile(&self, verbose: Output) -> miette::Result<()> {
        find_bin_manifest(&self.config.bin)
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        let mut features = vec![];
        if self.config.alsa {
            features.push("alsa");
        }
        if self.config.rerun {
            features.push("rerun");
        }
        if self.config.local {
            features.push("local");
        }

        let target = if self.config.local {
            None
        } else {
            Some(ROBOT_TARGET)
        };

        let pb = if matches!(verbose, Output::Verbose) {
            let pb = ProgressBar::new_spinner();
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
            Some(pb)
        } else {
            None
        };

        cargo::build(
            &self.config.bin,
            Profile::Release,
            target,
            &features,
            Some(cross::ENV_VARS.to_vec()),
        )
        .await?;

        if let Some(pb) = pb {
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

        let release_path = if self.config.local {
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

    /// Upload the binary, and other assets to each robot
    pub async fn upload(&self, verbose: Output) -> miette::Result<()> {
        let robot_numbers = self.config.robots.robot_numbers();
        if robot_numbers.len() == 1 {
            let robot = self.get_robot(robot_numbers[0])?;
            single_upload(robot, verbose, None).await.unwrap();
            return Ok(());
        }

        let mpb = MultiProgress::new();
        mpb.println(format!("   {}", "Deploying".green().bold()))
            .into_diagnostic()?;

        let mut set = tokio::task::JoinSet::new();
        for robot in robot_numbers {
            let robot = self.get_robot(robot)?;
            let pb = mpb.clone();

            let npb = pb.add(ProgressBar::new(13));
            npb.set_style(ProgressStyle::with_template("   {prefix:.blue.bold} {msg}").unwrap());

            pb.println("added new pb");
            npb.tick();
            let cnpb = npb.clone();
            set.spawn(async move {
                single_upload(robot, verbose.clone(), Some(cnpb))
                    .await
                    .unwrap()
            });
        }

        while let Some(res) = set.join_next().await {
            mpb.println("done with task").into_diagnostic()?;
            res.into_diagnostic()?;
        }
        mpb.clear().into_diagnostic()?;
        Ok(())
    }

    /// Get a specific robot
    pub fn get_robot(&self, robot: u8) -> miette::Result<Robot> {
        self.sindri_config
            .robot(robot, self.config.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                robot
            )))
    }

    /// Get robot information for a single robot, when there is just a single robot
    pub fn get_first_robot(&self) -> miette::Result<Robot> {
        if self.config.robots.is_empty() {
            return Err(miette!("Pass at least one robot number as argument"));
        }

        self.get_robot(self.config.robots[0].robot_number)
    }

    /// Start the yggdrasil service on each robot
    pub async fn start_yggdrasil_services(&self) -> miette::Result<()> {
        let mut threads: Vec<_> = Vec::new();

        for robot in self.config.robots.robot_numbers() {
            let robot = self.get_robot(robot)?;
            let thread = tokio::spawn(async move {
                start_single_yggdrasil_service(robot).await.unwrap();
            });
            threads.push(thread);
        }

        for temp_thread in threads {
            temp_thread.await.into_diagnostic()?;
        }

        Ok(())
    }

    /// Stop the yggdrasil service on each robot
    pub async fn stop_yggdrasil_services(&self) -> miette::Result<()> {
        let mut threads: Vec<_> = vec![];

        for robot in self.config.robots.robot_numbers() {
            let robot = self.get_robot(robot)?;
            let thread = tokio::spawn(async move {
                stop_single_yggdrasil_service(robot).await.unwrap();
            });
            threads.push(thread);
        }

        for temp_thread in threads {
            temp_thread.await.into_diagnostic()?;
        }

        Ok(())
    }
}

/// Upload the binary, and other assets to a specific robot
async fn single_upload(
    robot: Robot,
    verbose: Output,
    // mpb: Option<MultiProgress>,
    pb: Option<ProgressBar>,
) -> miette::Result<()> {
    find_bin_manifest(BINARY_NAME)
        .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

    match (verbose, pb) {
        (Output::Silent, _) => {
            upload_to_robot(SindriProgressBar::None, robot.ip())
                .await
                .wrap_err("Failed to deploy yggdrasil files to robot")?;
        }
        (Output::Verbose, Some(pb)) => {
            // mpb.println(format!("{:?}", mpb.clone()));
            // mpb.println(format!("num: {}", mpb.add(pb)))
            // mpb.println(format!("running deployment to {}", robot.ip()));
            upload_to_robot(SindriProgressBar::Multi(pb.clone()), robot.ip())
                .await
                .wrap_err("Failed to deploy yggdrasil files to robot")?;

            pb.finish_and_clear();
        }
        (Output::Verbose, _) => {
            let pb = ProgressBar::new(1);
            // pb.enable_steady_tick(Duration::from_millis(80));
            pb.set_style(
                ProgressStyle::with_template("   {prefix:.blue.bold} {msg} {spinner:.blue.bold}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            pb.set_prefix("Deploying");
            pb.set_message(format!("{}", "Preparing deployment...".dimmed()));

            upload_to_robot(SindriProgressBar::Single(pb.clone()), robot.ip())
                .await
                .wrap_err("Failed to deploy yggdrasil files to robot")?;

            pb.println(format!(
                "{} in {}",
                "  Uploaded to robot".bold(),
                HumanDuration(pb.elapsed()),
            ));

            pb.finish_and_clear();
        }
        _ => {
            return Err(miette!("Invalid combination of arguments!"));
        }
    }

    Ok(())
}

/// Modify the default network for a specific robot
pub async fn change_single_network(robot: Robot, network: String) -> miette::Result<()> {
    robot
        .ssh(
            format!("echo {} > /etc/network_config", network),
            Vec::<(&str, &str)>::new(),
            true,
        )?
        .wait()
        .await
        .into_diagnostic()?;

    robot
        .ssh(
            "sudo nohup systemctl restart network_config.service &> /dev/null",
            Vec::<(&str, &str)>::new(),
            true,
        )?
        .wait()
        .await
        .into_diagnostic()?;

    Ok(())
}

/// Start the yggdrasil service on a specific robot
async fn start_single_yggdrasil_service(robot: Robot) -> miette::Result<()> {
    robot
        .ssh(
            "sudo systemctl restart yggdrasil",
            Vec::<(&str, &str)>::new(),
            true,
        )?
        .wait()
        .await
        .into_diagnostic()?;

    Ok(())
}

/// Stop the yggdrasil service on a specific robot
async fn stop_single_yggdrasil_service(robot: Robot) -> miette::Result<()> {
    robot
        .ssh(
            "sudo systemctl stop yggdrasil",
            Vec::<(&str, &str)>::new(),
            true,
        )?
        .wait()
        .await
        .into_diagnostic()?;

    Ok(())
}

/// Copy the contents of the 'deploy' folder to the robot.
async fn upload_to_robot(pb: SindriProgressBar, addr: Ipv4Addr) -> Result<()> {
    match &pb {
        SindriProgressBar::None => {}
        SindriProgressBar::Multi(pb) => {

            // pb.set_message(format!(
            //     "{} {} {}",
            //     "  Connecting".bright_blue().bold(),
            //     "to".dimmed(),
            //     addr.to_string().clone().bold(),
            // ));
        }
        SindriProgressBar::Single(pb) => {
            pb.println(format!(
                "{} {} {}",
                "  Connecting".bright_blue().bold(),
                "to".dimmed(),
                addr.to_string().clone().bold(),
            ));
        }
    }

    let sftp = create_sftp_connection(addr).await?;

    let entries: Vec<DirEntry> = WalkDir::new("./deploy")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().unwrap().is_file())
        .collect();
    let num_files = entries.len();

    match &pb {
        SindriProgressBar::None => {}
        SindriProgressBar::Multi(pb) => {
            pb.set_length(num_files as u64);
            pb.set_style(
                ProgressStyle::with_template("   {prefix:.blue.bold} [{bar:.green/cyan}]: {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            pb.set_message(format!("{}", "Deploying".dimmed()));
            pb.tick();
        }
        SindriProgressBar::Single(pb) => {
            pb.set_message(format!("{}", "Ensuring host directories exist".dimmed()));

            pb.set_style(
                ProgressStyle::with_template(
                    "   {prefix:.blue.bold} {msg} [{bar:.blue/cyan}] {spinner:.blue.bold}",
                )
                .unwrap()
                // .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .progress_chars("=>-"),
            );
        }
    }

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
            .map_err(|e| Error::SftpError {
                source: e,
                msg: format!("Failed to open remote file {:?}!", entry.path()),
            })?;

        let mut file_local = std::fs::File::open(entry.path())?;

        match &pb {
            SindriProgressBar::None => {}
            SindriProgressBar::Multi(pb) => {
                // pb.set_message(format!(
                //     "{} {}",
                //     addr.to_string().red(),
                //     entry.path().to_string_lossy().dimmed()
                // ));
                pb.tick();
                // pb.inc(1);
            }
            SindriProgressBar::Single(pb) => {
                pb.set_length(file_local.metadata()?.len());
                pb.set_message(format!("{}", entry.path().to_string_lossy()));
            }
        }

        // Since `file_remote` impl's Write, we can just copy directly using a BufWriter!
        // The Write impl is rather slow, so we set a large buffer size of 1 mb.
        let mut buf_writer = BufWriter::with_capacity(UPLOAD_BUFFER_SIZE, file_remote);

        match &pb {
            SindriProgressBar::None => {
                std::io::copy(&mut file_local, &mut buf_writer)?;
            }
            SindriProgressBar::Multi(pb) => {
                std::io::copy(&mut file_local, &mut buf_writer)?;
                pb.inc(1);
            }
            SindriProgressBar::Single(pb) => {
                std::io::copy(&mut file_local, &mut pb.wrap_write(buf_writer))
                    .map_err(Error::IoError)?;

                pb.println(format!(
                    "{} {}",
                    "    Uploaded".bright_blue().bold(),
                    entry.path().to_string_lossy().dimmed()
                ));
            }
        }
    }

    // if let SindriProgressBar::Multi(mpb, _) = &pb {
    //     mpb.println(format!(
    //         "    {} {}",
    //         "Uploaded".green().bold(),
    //         addr.to_string().red()
    //     ))?;
    // }

    Ok(())
}

async fn create_sftp_connection(ip: Ipv4Addr) -> Result<Sftp> {
    let tcp = tokio::time::timeout(
        Duration::from_secs(CONNECTION_TIMEOUT),
        TcpStream::connect(format!("{ip}:22")),
    )
    .await
    .map_err(Error::ElapsedError)??;
    let mut session = Session::new().map_err(|e| Error::SftpError {
        source: e,
        msg: "Failed to create ssh session!".to_owned(),
    })?;

    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| Error::SftpError {
        source: e,
        msg: "Failed to perform ssh handshake!".to_owned(),
    })?;
    session
        .userauth_password("nao", "")
        .map_err(|e| Error::SftpError {
            source: e,
            msg: "Failed to authenticate using ssh!".to_owned(),
        })?;

    session.sftp().map_err(|e| Error::SftpError {
        source: e,
        msg: "Failed to create sftp session!".to_owned(),
    })
}

fn ensure_directory_exists(sftp: &Sftp, remote_path: impl AsRef<Path>) -> Result<()> {
    match sftp.mkdir(remote_path.as_ref(), 0o777) {
        Ok(()) => Ok(()),
        // Error code 4, means the directory already exists, so we can ignore it
        Err(error) if error.code() == ErrorCode::SFTP(4) => Ok(()),
        Err(error) => Err(Error::SftpError {
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
