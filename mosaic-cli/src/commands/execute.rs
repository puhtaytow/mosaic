use anyhow::{Result, anyhow, bail};
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;

use crate::{
    cli::{ClientConfig, ExecuteArgs},
    instructions::build_execute_instruction,
    models::{ExecuteBuild, ResolvedSession},
    rpc::{
        decode_instruction_accounts, load_additional_signers, load_default_signer, resolve_session,
        send_instruction,
    },
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

    let instruction = execute.instruction.clone();
    let signature = send_instruction(rpc, config, &payer, instruction, &additional_signers)
        .map_err(|error| humanize_execute_error(error, &resolved, &execute))?;

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

fn humanize_execute_error(
    error: anyhow::Error,
    resolved: &ResolvedSession,
    execute: &ExecuteBuild,
) -> anyhow::Error {
    let error_text = error.to_string();
    if !error_text.contains("insufficient account keys for instruction") {
        return error;
    }

    let stored_accounts = decode_instruction_accounts(&resolved.session.data)
        .unwrap_or_else(|_| execute.remaining_accounts.clone());
    let root_pda = resolved.root.pubkey.to_string();
    let root_in_session = stored_accounts
        .iter()
        .any(|account| account.pubkey == root_pda);
    let formatted_accounts = stored_accounts
        .iter()
        .map(|account| {
            format!(
                "{} [writable={}, signer={}]",
                account.pubkey, account.writable, account.signer
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let root_hint = if root_in_session {
        String::new()
    } else {
        format!(
            " Root PDA {} is not present in the stored instruction accounts. If the destination instruction expects the Mosaic root PDA as one of its CPI accounts, include it in the session account list with the correct signer/writable flags.",
            root_pda
        )
    };

    anyhow!(
        "failed to execute session {}: destination program reported insufficient account keys. The signing session stores {} instruction account(s): {}. CLI forwards only the CPI accounts stored in the session state, so the session was likely created with an incomplete or incorrect account list.{}",
        resolved.session.data.session_id,
        stored_accounts.len(),
        formatted_accounts,
        root_hint
    )
}
