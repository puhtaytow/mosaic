use anyhow::{Result, bail};
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;

use crate::{
    cli::{ClientConfig, ExecuteArgs},
    instructions::build_execute_instruction,
    rpc::{load_additional_signers, load_default_signer, resolve_session, send_instruction},
    util::{print_json, pubkeys_to_strings},
    views::ExecuteResultView,
};

pub(crate) fn execute_command(
    rpc: &RpcClient,
    config: &ClientConfig,
    args: ExecuteArgs,
) -> Result<()> {
    let payer = load_default_signer(config)?;
    let resolved = resolve_session(rpc, config, &args.session)?;
    let execute = build_execute_instruction(config, &payer.pubkey(), &resolved)?;
    let additional_signers = load_additional_signers(&args.additional_signers)?;
    let additional_signer_pubkeys: Vec<_> = additional_signers.iter().map(Signer::pubkey).collect();

    for required_signer in &execute.required_outer_signers {
        if *required_signer != payer.pubkey()
            && !additional_signer_pubkeys.contains(required_signer)
        {
            bail!(
                "session {} requires signature from {}; pass --additional-signer for that keypair",
                resolved.session.data.session_id,
                required_signer
            );
        }
    }

    let signature = send_instruction(
        rpc,
        config,
        &payer,
        execute.instruction,
        &additional_signers,
    )?;

    print_json(&ExecuteResultView {
        signature,
        program_id: config.program_id.to_string(),
        root_pda: resolved.root.pubkey.to_string(),
        session_pda: resolved.session.pubkey.to_string(),
        session_id: resolved.session.data.session_id,
        destination_program: execute.destination_program.to_string(),
        remaining_accounts: execute.remaining_accounts,
        additional_signers: pubkeys_to_strings(&additional_signer_pubkeys),
    })
}
