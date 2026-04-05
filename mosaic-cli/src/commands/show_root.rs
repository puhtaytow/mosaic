use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;

use crate::{
    cli::ClientConfig,
    rpc::fetch_root,
    util::{address_to_pubkey, addresses_to_strings, print_json},
    views::RootView,
};

pub(crate) fn show_root_command(rpc: &RpcClient, config: &ClientConfig) -> Result<()> {
    let root = match fetch_root(rpc, config) {
        Ok(ok) => ok,
        Err(err) => return Err(err),
    };

    print_json(&RootView {
        program_id: config.program_id.to_string(),
        root_pda: root.pubkey.to_string(),
        bump: root.data.bump,
        last_id: root.data.last_id,
        threshold: root.data.threshold,
        destination_program: address_to_pubkey(&root.data.destination_program).to_string(),
        operators: addresses_to_strings(&root.data.operators),
    })
}
