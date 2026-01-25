mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    ROOT_PDA,
    errors::MosaicError,
    instructions::{Instruction as ProgramIx, init_root::InitializeRootIxData},
};

use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[test]
fn test_initialize_root_not_writable_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
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
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountMustBeWrittable as u32,
        ))],
    );
}

#[test]
fn test_re_initialize_root_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 0;
    let (
        root_pda,
        root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators.clone(),
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

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
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::err(ProgramError::Custom(
                MosaicError::RootAccountMustNotBeInitialized as u32,
            )),
            Check::account(&root_pda).owner(&PROGRAM_ID).build(),
        ],
    );
}

#[test]
fn test_initialize_root_payer_is_not_signer_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (payer, signer_account) = operators.operators[0].clone();

    let session_id = 0;
    let (
        root_pda,
        root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators.clone(),
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

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
            AccountMeta::new(payer.into(), false),
            AccountMeta::new(root_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (payer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::err(ProgramError::Custom(
                MosaicError::PayerMustEqualSigner as u32,
            )),
            Check::account(&root_pda).owner(&PROGRAM_ID).build(),
        ],
    );
}

#[test]
fn test_initialize_root_threshold_higher_than_operators_count_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let amount_of_operators = 3;
    let operators = Operators::new(amount_of_operators, system_program);
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
        threshold: amount_of_operators + 1, // issue here
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
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::ThresholdCanNotBeHigherThanLenOfOperators as u32,
        ))],
    );
}

#[test]
fn test_initialize_root_zero_operators_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(1, system_program);
    let (signer, signer_account) = operators.operators[0].clone();

    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data = InitializeRootIxData {
        operators: vec![], // issue here
        threshold: 1,
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
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::OperatorsCountMustBePositive as u32,
        ))],
    );
}

#[test]
fn test_initialize_root_zero_threshold_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(1, system_program);
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
        operators: operators_pubkey,
        threshold: 0, // issue here
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
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::ThresholdCanNotBeZero as u32,
        ))],
    );
}

#[test]
fn test_initialize_root_over_max_operators_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(21, system_program);
    let (signer, signer_account) = operators.operators[0].clone();
    let operators_pubkey: Vec<Pubkey> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();

    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data = InitializeRootIxData {
        operators: operators_pubkey, // issue here
        threshold: 1,
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
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::ReachedOperatorsLimit as u32,
        ))],
    );
}
