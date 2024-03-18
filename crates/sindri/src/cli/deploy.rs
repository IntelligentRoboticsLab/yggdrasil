use crate::{
    cargo::{self, Profile},
    config::Config,
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
const RELEASE_PATH: &str = "./target/x86_64-unknown-linux-gnu/release/yggdrasil";
const DEPLOY_PATH: &str = "./deploy/yggdrasil";

/// The size of the `BufWriter`'s buffer.
///
/// This is currently set to 1 MiB, as the [`Write`] implementation for [`ssh2::sftp::File`]
/// is rather slow due to the locking mechanism.
const UPLOAD_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsDeploy {
    /// Number of the robot to deploy to.
    #[clap(index = 1, name = "robot-number")]
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
}

impl ConfigOptsDeploy {
    #[must_use]
    pub fn new(number: u8, wired: bool, team_number: Option<u8>, rerun: bool) -> Self {
        Self {
            number,
            wired,
            team_number,
            rerun,
        }
    }
}

/// Compile and deploy the specified binary to the robot.
#[derive(Parser)]
pub struct Deploy {
    #[clap(flatten)]
    pub deploy: ConfigOptsDeploy,
}

impl Deploy {
    /// Constructs IP and deploys to the robot
    pub async fn deploy(self, config: Config) -> miette::Result<()> {
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

        let mut features = vec!["alsa"];
        if self.deploy.rerun {
            features.push("rerun");
        }

        // Build yggdrasil with cargo
        cargo::build("yggdrasil", Profile::Release, Some(ROBOT_TARGET), features).await?;

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

        // Check if the robot exists
        let robot = config
            .robot(self.deploy.number, self.deploy.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.deploy.number
            )))?;

        pb.set_style(
            ProgressStyle::with_template("   {prefix:.blue.bold} {msg} {spinner:.blue.bold}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );

        pb.set_prefix("Deploying");
        pb.set_message(format!("{}", "Preparing deployment...".dimmed()));

        // Copy over the files that need to be deployed
        fs::copy(RELEASE_PATH, DEPLOY_PATH)
            .into_diagnostic()
            .wrap_err("Failed to copy binary to deploy directory!")?;

        deploy_to_robot(&pb, robot.ip())
            .await
            .wrap_err("Failed to deploy yggdrasil files to robot")?;

        pb.println(format!(
            "{} in {}",
            "  Deployed to robot".bold(),
            HumanDuration(pb.elapsed()),
        ));
        pb.finish_and_clear();

        Ok(())
    }
}

/// Copy the contents of the 'deploy' folder to the robot.
async fn deploy_to_robot(pb: &ProgressBar, addr: Ipv4Addr) -> Result<()> {
    pb.println(format!(
        "{} {} {}",
        "  Connecting".bright_blue().bold(),
        "to".dimmed(),
        addr.to_string().clone().bold(),
    ));

    let sftp = create_sftp_connection(addr).await?;

    pb.set_message(format!("{}", "Ensuring host directories exist".dimmed()));

    // Ensure asset directory and sounds directory exist on remote
    ensure_directory_exists(&sftp, "/home/nao/assets")?;
    ensure_directory_exists(&sftp, "/home/nao/assets/sounds")?;
    ensure_directory_exists(&sftp, "/home/nao/assets/motions")?;

    pb.set_style(
        ProgressStyle::with_template(
            "   {prefix:.blue.bold} {msg} [{bar:.blue/cyan}] {spinner:.blue.bold}",
        )
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .progress_chars("=>-"),
    );

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
        pb.set_length(file_length);
        pb.set_message(format!("{}", entry.path().to_string_lossy()));

        let buf_writer = BufWriter::with_capacity(UPLOAD_BUFFER_SIZE, file_remote);
        std::io::copy(&mut file_local, &mut pb.wrap_write(buf_writer)).map_err(Error::IoError)?;

        pb.println(format!(
            "{} {}",
            "    Uploaded".bright_blue().bold(),
            entry.path().to_string_lossy().dimmed()
        ));
    }

    Ok(())
}

async fn create_sftp_connection(ip: Ipv4Addr) -> Result<Sftp> {
    let tcp = tokio::time::timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("{ip}:22")),
    )
    .await
    .map_err(Error::ElapsedError)?
    .unwrap();
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
