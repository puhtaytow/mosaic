use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use solana_rpc_client::rpc_client::RpcClient;

use crate::{
    cli::{ClientConfig, SessionSelectorArgs},
    instructions::build_execute_remaining_accounts,
    rpc::{decode_instruction_accounts, resolve_session},
    util::{
        address_to_pubkey, addresses_to_strings, encode_hex, phase_to_string, print_json,
        pubkeys_to_strings,
    },
    views::SessionView,
};

pub(crate) fn show_session_command(
    rpc: &RpcClient,
    config: &ClientConfig,
    selector: &SessionSelectorArgs,
) -> Result<()> {
    let resolved = resolve_session(rpc, config, selector)?;
    let execute_plan =
        build_execute_remaining_accounts(&resolved.session.data, &resolved.root.pubkey)?;

    print_json(&SessionView {
        program_id: config.program_id.to_string(),
        root_pda: resolved.root.pubkey.to_string(),
        root_last_id: resolved.root.data.last_id,
        destination_program: address_to_pubkey(&resolved.root.data.destination_program).to_string(),
        threshold: resolved.root.data.threshold,
        session_pda: resolved.session.pubkey.to_string(),
        session_id: resolved.session.data.session_id,
        bump: resolved.session.data.bump,
        phase: phase_to_string(resolved.session.data.phase).to_owned(),
        is_latest: resolved.session.data.session_id == resolved.root.data.last_id,
        approvals: addresses_to_strings(&resolved.session.data.approvals),
        approvals_count: resolved.session.data.approvals.len(),
        instruction_data_hex: encode_hex(&resolved.session.data.instruction_data),
        instruction_data_base64: BASE64_STANDARD.encode(&resolved.session.data.instruction_data),
        instruction_data_utf8: String::from_utf8(resolved.session.data.instruction_data.clone())
            .ok(),
        instruction_accounts: decode_instruction_accounts(&resolved.session.data)?,
        execute_root_writable: execute_plan.root_writable,
        execute_remaining_accounts: execute_plan.remaining_accounts,
        execute_required_outer_signers: pubkeys_to_strings(&execute_plan.required_outer_signers),
    })
}
