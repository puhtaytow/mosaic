mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    ROOT_PDA,
    instructions::{Instruction as ProgramIx, init_root::InitializeRootIxData},
    state::root::Root,
};

use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

#[test]
fn test_initialize_root() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<Pubkey> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data = InitializeRootIxData {
        operators: operators_pubkey.clone(),
        threshold: operators.threshold,
        bump: root_pda_bump,
        destination_program: DESTINATION_PROGRAM_ID,
    };
    let data = [
        vec![ProgramIx::InitializeOperators as u8],
        to_vec(&ix_data).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::success(),
            Check::account(&root_pda).owner(&PROGRAM_ID).build(),
        ],
    );
    let updated_root_pda_account = result.get_account(&root_pda).unwrap();
    let parsed_root_pda_data = borsh::from_slice::<Root>(&updated_root_pda_account.data).unwrap();

    assert!(parsed_root_pda_data.bump == root_pda_bump);
    assert!(parsed_root_pda_data.last_id == 0);
    assert!(parsed_root_pda_data.threshold == operators.threshold);
    assert!(parsed_root_pda_data.operators == operators_pubkey);
}

#[test]
fn test_initialize_root_huge_operators_list() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(20, system_program);
    let operators_pubkey: Vec<Pubkey> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data = InitializeRootIxData {
        operators: operators_pubkey.clone(),
        threshold: operators.threshold,
        bump: root_pda_bump,
        destination_program: DESTINATION_PROGRAM_ID,
    };
    let data = [
        vec![ProgramIx::InitializeOperators as u8],
        to_vec(&ix_data).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::success(),
            Check::account(&root_pda).owner(&PROGRAM_ID).build(),
        ],
    );
    let updated_root_pda_account = result.get_account(&root_pda).unwrap();
    let parsed_root_pda_data = borsh::from_slice::<Root>(&updated_root_pda_account.data).unwrap();

    assert!(parsed_root_pda_data.bump == root_pda_bump);
    assert!(parsed_root_pda_data.last_id == 0);
    assert!(parsed_root_pda_data.threshold == operators.threshold);
    assert!(parsed_root_pda_data.operators == operators_pubkey);
}
