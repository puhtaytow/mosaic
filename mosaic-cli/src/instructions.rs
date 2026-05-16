use anyhow::{Result, anyhow};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use std::collections::HashMap;

use crate::{
    cli::{ClientConfig, InitRootArgs},
    models::{
        ExecuteBuild, ExecutePlan, InstructionAccountView, LoadedRoot, LoadedSessionSpec,
        ResolvedSession,
    },
    util::{account_meta, address_to_pubkey, instruction_data, pubkey_to_address},
};
use mosaic::{
    ROOT_PDA, SIGNING_SESSION_PDA,
    instructions::{
        Instruction as MosaicInstruction, close_session_account::CloseSessionAccountIxData,
        execute::ExecuteIxData, init_root::InitializeRootIxData,
        init_signing_session::InitializeSigningSessionIxData, sign::SignIxData,
    },
    state::signing_session::{InstructionAccount as SessionInstructionAccount, SigningSession},
};

pub(crate) fn build_init_root_instruction(
    config: &ClientConfig,
    payer: &Pubkey,
    args: &InitRootArgs,
) -> Result<(Instruction, Pubkey, u8)> {
    let (root_pda, bump) = derive_root_pda(&config.program_id);
    let ix_data = InitializeRootIxData {
        operators: args.operators.iter().map(pubkey_to_address).collect(),
        threshold: args.threshold,
        destination_program: pubkey_to_address(&args.destination_program),
        bump,
    };

    let instruction = Instruction {
        program_id: config.program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: instruction_data(MosaicInstruction::InitializeOperators as u8, &ix_data)?,
    };

    Ok((instruction, root_pda, bump))
}

pub(crate) fn build_init_session_instruction(
    config: &ClientConfig,
    payer: &Pubkey,
    root: &LoadedRoot,
    next_session_id: u16,
    spec: &LoadedSessionSpec,
) -> Result<(Instruction, Pubkey, u8)> {
    let (session_pda, bump) =
        derive_signing_session_pda(&config.program_id, &root.pubkey, next_session_id);
    let ix_data = InitializeSigningSessionIxData {
        instruction_data: spec.instruction_data.clone(),
        instruction_accounts: spec.instruction_accounts.clone(),
        bump,
    };

    let instruction = Instruction {
        program_id: config.program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(root.pubkey, false),
            AccountMeta::new(session_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: instruction_data(MosaicInstruction::InitializeSigningSession as u8, &ix_data)?,
    };

    Ok((instruction, session_pda, bump))
}

pub(crate) fn build_sign_instruction(
    config: &ClientConfig,
    payer: &Pubkey,
    resolved: &ResolvedSession,
) -> Result<Instruction> {
    let ix_data = SignIxData {
        bump: resolved.session.data.bump,
    };

    Ok(Instruction {
        program_id: config.program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(resolved.root.pubkey, false),
            AccountMeta::new(resolved.session.pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: instruction_data(MosaicInstruction::Sign as u8, &ix_data)?,
    })
}

pub(crate) fn build_execute_instruction(
    config: &ClientConfig,
    payer: &Pubkey,
    resolved: &ResolvedSession,
) -> Result<ExecuteBuild> {
    let execute_plan =
        build_execute_remaining_accounts(&resolved.session.data, &resolved.root.pubkey)?;
    let destination_program = address_to_pubkey(&resolved.root.data.destination_program);
    let root_meta = if execute_plan.root_writable {
        AccountMeta::new(resolved.root.pubkey, false)
    } else {
        AccountMeta::new_readonly(resolved.root.pubkey, false)
    };

    let mut accounts = vec![
        AccountMeta::new(*payer, true),
        root_meta,
        AccountMeta::new(resolved.session.pubkey, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(destination_program, false),
    ];
    accounts.extend(execute_plan.account_metas.clone());

    Ok(ExecuteBuild {
        instruction: Instruction {
            program_id: config.program_id,
            accounts,
            data: instruction_data(MosaicInstruction::Execute as u8, &ExecuteIxData {})?,
        },
        remaining_accounts: execute_plan.remaining_accounts,
        required_outer_signers: execute_plan.required_outer_signers,
        destination_program,
    })
}

pub(crate) fn build_close_session_instruction(
    config: &ClientConfig,
    payer: &Pubkey,
    resolved: &ResolvedSession,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id: config.program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(resolved.root.pubkey, false),
            AccountMeta::new(resolved.session.pubkey, false),
        ],
        data: instruction_data(
            MosaicInstruction::CloseSessionAccount as u8,
            &CloseSessionAccountIxData {},
        )?,
    })
}

pub(crate) fn build_execute_remaining_accounts(
    session: &SigningSession,
    root_pda: &Pubkey,
) -> Result<ExecutePlan> {
    let mut root_writable = false;
    let mut merged: Vec<(Pubkey, bool, bool)> = Vec::new();
    let mut index_by_pubkey: HashMap<Pubkey, usize> = HashMap::new();
    let mut required_outer_signers = Vec::new();

    for raw in &session.instruction_accounts {
        let account = SessionInstructionAccount::deserialize(raw)
            .map_err(|error| anyhow!("failed to decode stored instruction account: {error:?}"))?;
        let pubkey = Pubkey::new_from_array(account.pubkey);

        if account.signer && pubkey != *root_pda && !required_outer_signers.contains(&pubkey) {
            required_outer_signers.push(pubkey);
        }

        if pubkey == *root_pda {
            root_writable |= account.writable;
            continue;
        }

        if let Some(index) = index_by_pubkey.get(&pubkey).copied() {
            let entry = &mut merged[index];
            entry.1 |= account.writable;
            entry.2 |= account.signer;
            continue;
        }

        index_by_pubkey.insert(pubkey, merged.len());
        merged.push((pubkey, account.writable, account.signer));
    }

    let remaining_accounts: Vec<InstructionAccountView> = merged
        .iter()
        .map(|(pubkey, writable, signer)| InstructionAccountView {
            pubkey: pubkey.to_string(),
            writable: *writable,
            signer: *signer,
        })
        .collect();
    let account_metas = merged
        .into_iter()
        .map(|(pubkey, writable, signer)| account_meta(pubkey, writable, signer))
        .collect();

    Ok(ExecutePlan {
        root_writable,
        account_metas,
        remaining_accounts,
        required_outer_signers,
    })
}

pub(crate) fn derive_root_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROOT_PDA], program_id)
}

pub(crate) fn derive_signing_session_pda(
    program_id: &Pubkey,
    root_pda: &Pubkey,
    session_id: u16,
) -> (Pubkey, u8) {
    let session_id_bytes = session_id.to_be_bytes();
    Pubkey::find_program_address(
        &[root_pda.as_ref(), &session_id_bytes, SIGNING_SESSION_PDA],
        program_id,
    )
}
