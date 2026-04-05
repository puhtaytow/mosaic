use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;

use crate::{
    cli::{ClientConfig, InitRootArgs},
    instructions::build_init_root_instruction,
    rpc::{load_default_signer, send_instruction},
    util::{print_json, pubkeys_to_strings},
    views::InitRootResultView,
};

pub(crate) fn init_root_command(
    rpc: &RpcClient,
    config: &ClientConfig,
    args: InitRootArgs,
) -> Result<()> {
    let payer = load_default_signer(config)?;
    let (instruction, root_pda, bump) =
        build_init_root_instruction(config, &payer.pubkey(), &args)?;
    let signature = send_instruction(rpc, config, &payer, instruction, &[])?;

    print_json(&InitRootResultView {
        signature,
        program_id: config.program_id.to_string(),
        root_pda: root_pda.to_string(),
        bump,
        threshold: args.threshold,
        destination_program: args.destination_program.to_string(),
        operators: pubkeys_to_strings(&args.operators),
    })
}
