use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::util::expand_path;

#[derive(Parser, Debug)]
#[command(name = "mosaic-cli", version, about = "CLI for the Mosaic program")]
pub(crate) struct Cli {
    #[arg(long, default_value = "config.json", value_name = "PATH")]
    pub(crate) config: PathBuf,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    ShowRoot,
    ListSessions,
    ShowSession {
        #[command(flatten)]
        session: SessionSelectorArgs,
    },
    InitRoot(InitRootArgs),
    InitSession(InitSessionArgs),
    Sign {
        #[command(flatten)]
        session: SessionSelectorArgs,
    },
    Execute(ExecuteArgs),
    CloseSession {
        #[command(flatten)]
        session: SessionSelectorArgs,
    },
}

#[derive(Args, Debug)]
pub(crate) struct InitRootArgs {
    #[arg(long, value_parser = parse_pubkey, num_args = 1..)]
    pub(crate) operators: Vec<Pubkey>,

    #[arg(long)]
    pub(crate) threshold: u8,

    #[arg(long, value_parser = parse_pubkey)]
    pub(crate) destination_program: Pubkey,
}

#[derive(Args, Debug)]
pub(crate) struct InitSessionArgs {
    #[arg(long, value_name = "PATH", conflicts_with = "data")]
    pub(crate) spec: Option<PathBuf>,

    #[arg(long, value_name = "HEX_OR_BASE64_OR_TEXT", conflicts_with = "spec")]
    pub(crate) data: Option<String>,

    #[arg(long, value_name = "HEX_ACCOUNT", requires = "data")]
    pub(crate) accounts: Vec<String>,

    #[arg(long, value_enum, default_value_t = DataEncodingArg::Hex, requires = "data")]
    pub(crate) data_encoding: DataEncodingArg,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct SessionSelectorArgs {
    #[arg(long, value_parser = parse_pubkey, conflicts_with = "session_id")]
    pub(crate) session: Option<Pubkey>,

    #[arg(long)]
    pub(crate) session_id: Option<u16>,
}

#[derive(Args, Debug)]
pub(crate) struct ExecuteArgs {
    #[command(flatten)]
    pub(crate) session: SessionSelectorArgs,

    #[arg(long = "additional-signer", value_name = "PATH")]
    pub(crate) additional_signers: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CommitmentArg {
    Processed,
    Confirmed,
    Finalized,
}

impl CommitmentArg {
    fn into_config(self) -> CommitmentConfig {
        match self {
            CommitmentArg::Processed => CommitmentConfig::processed(),
            CommitmentArg::Confirmed => CommitmentConfig::confirmed(),
            CommitmentArg::Finalized => CommitmentConfig::finalized(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DataEncodingArg {
    Hex,
    Base64,
    Utf8,
}

#[derive(Clone)]
pub(crate) struct ClientConfig {
    pub(crate) rpc_url: String,
    pub(crate) keypair_path: PathBuf,
    pub(crate) commitment: CommitmentConfig,
    pub(crate) program_id: Pubkey,
}

impl ClientConfig {
    pub(crate) fn from_cli(cli: &Cli) -> Result<Self> {
        let config_path = expand_path(&cli.config)?;
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        let config_file = load_config_file(&config_path)?;

        Ok(Self {
            rpc_url: config_file.rpc_url,
            keypair_path: resolve_config_path(config_dir, &config_file.keypair)?,
            commitment: config_file
                .commitment
                .unwrap_or(CommitmentArg::Confirmed)
                .into_config(),
            program_id: parse_optional_pubkey(config_file.program_id, &config_path, "program_id")?
                .unwrap_or_else(default_program_id),
        })
    }
}

#[derive(Deserialize)]
struct ClientConfigFile {
    rpc_url: String,
    keypair: PathBuf,
    #[serde(default)]
    program_id: Option<String>,
    #[serde(default)]
    commitment: Option<CommitmentArg>,
}

fn load_config_file(path: &Path) -> Result<ClientConfigFile> {
    let file = File::open(path)
        .with_context(|| format!("failed to open config file {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse config file {}", path.display()))
}

fn resolve_config_path(base_dir: &Path, path: &Path) -> Result<PathBuf> {
    let path = expand_path(path)?;
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(base_dir.join(path))
    }
}

fn parse_optional_pubkey(
    value: Option<String>,
    config_path: &Path,
    field_name: &str,
) -> Result<Option<Pubkey>> {
    value
        .map(|value| {
            Pubkey::from_str(&value).map_err(|error| {
                anyhow!(
                    "invalid {field_name} `{value}` in config file {}: {error}",
                    config_path.display()
                )
            })
        })
        .transpose()
}

fn parse_pubkey(value: &str) -> std::result::Result<Pubkey, String> {
    Pubkey::from_str(value).map_err(|error| error.to_string())
}

fn default_program_id() -> Pubkey {
    Pubkey::new_from_array(mosaic::ID)
}
