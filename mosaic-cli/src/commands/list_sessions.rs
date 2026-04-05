use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;

use crate::{
    cli::ClientConfig,
    rpc::{fetch_root, list_sessions},
    util::{addresses_to_strings, phase_to_string, print_json},
    views::{ListSessionsView, SessionSummaryView},
};

pub(crate) fn list_sessions_command(rpc: &RpcClient, config: &ClientConfig) -> Result<()> {
    let root = fetch_root(rpc, config)?;
    let sessions = list_sessions(rpc, config, &root.pubkey)?
        .into_iter()
        .map(|(pubkey, session)| SessionSummaryView {
            session_pda: pubkey.to_string(),
            session_id: session.session_id,
            phase: phase_to_string(session.phase).to_owned(),
            approvals: addresses_to_strings(&session.approvals),
            approvals_count: session.approvals.len(),
            bump: session.bump,
        })
        .collect();

    print_json(&ListSessionsView {
        program_id: config.program_id.to_string(),
        root_pda: root.pubkey.to_string(),
        root_last_id: root.data.last_id,
        sessions,
    })
}
