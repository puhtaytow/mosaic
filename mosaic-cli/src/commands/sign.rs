use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;

use crate::{
    cli::{ClientConfig, SessionSelectorArgs},
    instructions::build_sign_instruction,
    rpc::{load_default_signer, resolve_session, send_instruction},
    util::print_json,
    views::SessionActionResultView,
};

pub(crate) fn sign_command(
    rpc: &RpcClient,
    config: &ClientConfig,
    selector: &SessionSelectorArgs,
) -> Result<()> {
    let payer = load_default_signer(config)?;
    let resolved = resolve_session(rpc, config, selector)?;
    let instruction = build_sign_instruction(config, &payer.pubkey(), &resolved)?;
    let signature = send_instruction(rpc, config, &payer, instruction, &[])?;

    print_json(&SessionActionResultView {
        action: "sign".to_owned(),
        signature,
        program_id: config.program_id.to_string(),
        root_pda: resolved.root.pubkey.to_string(),
        session_pda: resolved.session.pubkey.to_string(),
        session_id: resolved.session.data.session_id,
    })
}
