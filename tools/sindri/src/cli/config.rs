use std::{
    fs::{self, read_to_string, OpenOptions},
    io::{self, Write},
    path::Path,
};

use clap::Subcommand;
use dialoguer::{console::Style, theme::ColorfulTheme, Confirm};
use miette::{IntoDiagnostic, Result};

use crate::config::{load_config, SindriConfig};

const INIT_DIR: &str = "init/";

/// Generate or change the robot configuration
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Initialize a new sindri config and rerun the setup wizard
    Init,
    /// Opens the sindri config in the default text editor
    Open,
}

impl ConfigCommand {
    pub fn config(self) -> Result<()> {
        match self {
            ConfigCommand::Init => Self::init(),
            ConfigCommand::Open => Self::open(),
        }
    }

    pub fn init() -> Result<()> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let init_dir = Path::new(manifest_dir).join(INIT_DIR);
        let home_dir = home::home_dir().expect("Failed to get home directory");

        let setup_config = setup_wizard()?;

        install_sindri_config(&init_dir, &home_dir).into_diagnostic()?;

        let config = load_config()?;

        if setup_config.configure_ssh {
            install_ssh_keys(&home_dir, config).into_diagnostic()?;
        }

        println!("Finished sindri setup!");

        Ok(())
    }

    pub fn open() -> Result<()> {
        let editor = std::env::var("EDITOR").unwrap_or("code".to_string());
        let home_dir = home::home_dir().expect("Failed to get home directory");

        std::process::Command::new(editor)
            .arg(home_dir.join(".config/sindri/sindri.toml"))
            .spawn()
            .expect("Failed to open sindri config");

        Ok(())
    }
}

struct SetupConfig {
    configure_ssh: bool,
}

fn setup_wizard() -> Result<SetupConfig> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow(),
        ..ColorfulTheme::default()
    };

    println!("Welcome to the sindri setup wizard! 🚀");

    let configure_ssh = Confirm::with_theme(&theme)
        .with_prompt("Add robots to ssh config?")
        .default(true)
        .interact()
        .into_diagnostic()?;

    Ok(SetupConfig { configure_ssh })
}

fn install_sindri_config(init_dir: &Path, home_dir: &Path) -> io::Result<()> {
    copy_recursively(
        init_dir.join(".config").as_path(),
        home_dir.join(".config").as_path(),
    )
}

fn install_ssh_keys(home_dir: &Path, config: SindriConfig) -> io::Result<()> {
    fs::create_dir_all(home_dir.join(".ssh/nao"))?;

    // create ssh config using config toml
    let ssh_config = config
        .robots
        .iter()
        .map(|robot| {
            format!(
                concat!(
                    "Host {}\n",
                    "    Hostname 10.0.{}.{}\n",
                    "    user nao\n",
                    "    Port 22\n",
                ),
                robot.name, config.team_number, robot.number,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(
        format!("{}/.ssh/nao/config", home_dir.display()),
        ssh_config,
    )
    .expect("Unable to write ssh config");

    let ssh_config_path = home_dir.join(".ssh/config");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ssh_config_path)
        .expect("unable to open ssh config");

    let include_directive = "Match all\nInclude nao/config\n";
    let ssh_config = read_to_string(&ssh_config_path)?;

    if ssh_config.contains(include_directive) {
        Ok(())
    } else {
        f.write_all(include_directive.as_bytes())
    }
}

fn copy_recursively(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_path = destination.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_recursively(entry.path().as_path(), &file_path)?;
        } else {
            fs::copy(entry.path(), file_path)?;
        }
    }
    Ok(())
}
