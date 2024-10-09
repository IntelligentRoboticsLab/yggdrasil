use crate::{
    cargo::{self, find_bin_manifest, Profile},
    config::SindriConfig,
    error::{Error, Result},
};
use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use miette::{miette, Context, IntoDiagnostic};
use ssh2::{ErrorCode, OpenFlags, OpenType, Session, Sftp};
use std::{
    fs,
    io::BufWriter,
    net::Ipv4Addr,
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tokio::net::TcpStream;
use walkdir::WalkDir;

const ROBOT_TARGET: &str = "x86_64-unknown-linux-gnu";
const RELEASE_PATH_REMOTE: &str = "./target/x86_64-unknown-linux-gnu/release/yggdrasil";
const RELEASE_PATH_LOCAL: &str = "./target/release/yggdrasil";
const DEPLOY_PATH: &str = "./deploy/yggdrasil";

const LOCAL_ROBOT_ID_STR: &str = "0";

/// The size of the `BufWriter`'s buffer.
///
/// This is currently set to 1 MiB, as the [`Write`] implementation for [`ssh2::sftp::File`]
/// is rather slow due to the locking mechanism.
const UPLOAD_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ConfigOptsUpload {
    /// Number of the robot to deploy to.
    #[clap(
        index = 1,
        name = "robot-number",
        required(false),
        required_unless_present("local"),
        default_value_if("local", "true", Some(LOCAL_ROBOT_ID_STR)),
        conflicts_with("local")
    )]
    pub number: u8,

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
}

impl ConfigOptsUpload {
    #[must_use]
    pub fn new(
        number: u8,
        wired: bool,
        team_number: Option<u8>,
        rerun: bool,
        local: bool,
        alsa: bool,
        bin: String,
        silent: bool,
    ) -> Self {
        Self {
            number,
            wired,
            team_number,
            rerun,
            local,
            bin,
            alsa,
            silent,
        }
    }
}

/// Compile and deploy the specified binary to the robot.
pub struct Upload {
    pub deploy: ConfigOptsDeploy,
}

impl Deploy {
    /// Constructs IP and deploys to the robot
    pub async fn deploy(self, config: SindriConfig) -> miette::Result<()> {
        find_bin_manifest(&self.deploy.bin)
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        let mut pb: Option<ProgressBar> = None;
        if !self.deploy.silent {
            pb = Some(ProgressBar::new_spinner());
        }

        if let Some(pb) = &pb {
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
        }

        let mut features = vec![];
        if self.deploy.alsa {
            features.push("alsa");
        }
        if self.deploy.rerun {
            features.push("rerun");
        }
        if self.deploy.local {
            features.push("local");
        }

        let target = if self.deploy.local {
            None
        } else {
            Some(ROBOT_TARGET)
        };

        // Build yggdrasil with cargo
        cargo::build(
            "yggdrasil",
            Profile::Release,
            target,
            &features,
            Some(cross::ENV_VARS.to_vec()),
        )
        .await?;

        if let Some(pb) = &pb {
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

        let release_path = if self.deploy.local {
            RELEASE_PATH_LOCAL
        } else {
            RELEASE_PATH_REMOTE
        };

        // Copy over the files that need to be deployed
        fs::copy(release_path, DEPLOY_PATH)
            .into_diagnostic()
            .wrap_err("Failed to copy binary to deploy directory!")?;

        if self.deploy.local {
            return Ok(());
        }

        // Check if the robot exists
        let robot = config
            .robot(self.deploy.number, self.deploy.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.deploy.number
            )))?;

        if let Some(pb) = &pb {
            pb.set_style(
                ProgressStyle::with_template("   {prefix:.blue.bold} {msg} {spinner:.blue.bold}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );

            pb.set_prefix("Deploying");

            pb.set_message(format!("{}", "Preparing deployment...".dimmed()));
        }

        deploy_to_robot(pb.as_ref(), robot.ip())
            .await
            .wrap_err("Failed to deploy yggdrasil files to robot")?;

        if let Some(pb) = &pb {
            pb.println(format!(
                "{} in {}",
                "  Deployed to robot".bold(),
                HumanDuration(pb.elapsed()),
            ));
            pb.finish_and_clear();
        }

        Ok(())
    }
}

/// Copy the contents of the 'deploy' folder to the robot.
async fn deploy_to_robot(pb: Option<&ProgressBar>, addr: Ipv4Addr) -> Result<()> {
    if let Some(pb) = pb {
        pb.println(format!(
            "{} {} {}",
            "  Connecting".bright_blue().bold(),
            "to".dimmed(),
            addr.to_string().clone().bold(),
        ));
    }

    let sftp = create_sftp_connection(addr).await?;

    if let Some(pb) = pb {
        pb.set_message(format!("{}", "Ensuring host directories exist".dimmed()));

        pb.set_style(
            ProgressStyle::with_template(
                "   {prefix:.blue.bold} {msg} [{bar:.blue/cyan}] {spinner:.blue.bold}",
            )
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .progress_chars("=>-"),
        );
    }

    for entry in WalkDir::new("./deploy") {
        let entry = entry.unwrap();
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

        // Since `file_remote` impl's Write, we can just copy directly using a BufWriter!
        // The Write impl is rather slow, so we set a large buffer size of 1 mb.
        let file_length = file_local.metadata()?.len();

        if let Some(pb) = pb {
            pb.set_length(file_length);
            pb.set_message(format!("{}", entry.path().to_string_lossy()));
        }

        let mut buf_writer = BufWriter::with_capacity(UPLOAD_BUFFER_SIZE, file_remote);
        if let Some(pb) = pb {
            std::io::copy(&mut file_local, &mut pb.wrap_write(buf_writer))
                .map_err(Error::IoError)?;

            pb.println(format!(
                "{} {}",
                "    Uploaded".bright_blue().bold(),
                entry.path().to_string_lossy().dimmed()
            ));
        } else {
            std::io::copy(&mut file_local, &mut buf_writer)?;
        }
    }

    Ok(())
}

async fn create_sftp_connection(ip: Ipv4Addr) -> Result<Sftp> {
    let tcp = tokio::time::timeout(
        Duration::from_secs(5),
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
