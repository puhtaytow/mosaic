mod close_session;
mod execute;
mod init_root;
mod init_session;
mod list_sessions;
mod show_root;
mod show_session;
mod sign;

use anyhow::Result;
use clap::Parser;
use solana_rpc_client::rpc_client::RpcClient;

use crate::cli::{Cli, ClientConfig, Command};

pub(crate) fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = ClientConfig::from_cli(&cli)?;
    let rpc = RpcClient::new_with_commitment(config.rpc_url.clone(), config.commitment.clone());

    match cli.command {
        Command::ShowRoot => show_root::show_root_command(&rpc, &config),
        Command::ListSessions => list_sessions::list_sessions_command(&rpc, &config),
        Command::ShowSession { session } => {
            show_session::show_session_command(&rpc, &config, &session)
        }
        Command::InitRoot(args) => init_root::init_root_command(&rpc, &config, args),
        Command::InitSession(args) => init_session::init_session_command(&rpc, &config, args),
        Command::Sign { session } => sign::sign_command(&rpc, &config, &session),
        Command::Execute(args) => execute::execute_command(&rpc, &config, args),
        Command::CloseSession { session } => {
            close_session::close_session_command(&rpc, &config, &session)
        }
    }
}
