use anyhow::{Result, anyhow};
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;

use crate::{
    cli::{ClientConfig, InitSessionArgs},
    instructions::build_init_session_instruction,
    rpc::{
        fetch_root, load_default_signer, load_inline_session_spec, load_session_spec,
        send_instruction,
    },
    util::print_json,
    views::InitSessionResultView,
};

pub(crate) fn init_session_command(
    rpc: &RpcClient,
    config: &ClientConfig,
    args: InitSessionArgs,
) -> Result<()> {
    let payer = load_default_signer(config)?;
    let root = fetch_root(rpc, config)?;
    let next_session_id = root
        .data
        .last_id
        .checked_add(1)
        .ok_or_else(|| anyhow!("root last_id overflowed"))?;
    let spec = match (&args.spec, &args.data) {
        (Some(spec), None) => load_session_spec(spec)?,
        (None, Some(data)) => load_inline_session_spec(args.data_encoding, data, &args.accounts)?,
        (Some(_), Some(_)) => {
            return Err(anyhow!("pass either --spec or --data/--accounts, not both"));
        }
        (None, None) => {
            return Err(anyhow!("missing session input; pass --spec or --data"));
        }
    };
    let (instruction, session_pda, bump) =
        build_init_session_instruction(config, &payer.pubkey(), &root, next_session_id, &spec)?;
    let signature = send_instruction(rpc, config, &payer, instruction, &[])?;

    print_json(&InitSessionResultView {
        signature,
        program_id: config.program_id.to_string(),
        root_pda: root.pubkey.to_string(),
        session_pda: session_pda.to_string(),
        session_id: next_session_id,
        bump,
    })
}
