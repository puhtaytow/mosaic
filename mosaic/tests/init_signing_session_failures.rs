mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    ROOT_PDA, SIGNING_SESSION_PDA,
    errors::MosaicError,
    instructions::{
        Instruction as ProgramIx, init_signing_session::InitializeSigningSessionIxData,
    },
    state::signing_session::SigningSessionPhase,
};

use solana_program::example_mocks::{solana_keypair::Keypair, solana_signer::Signer};
use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    program_error::ProgramError,
};

#[test]
fn test_initialize_signing_session_root_pda_is_not_owned_by_program_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 0;
    let another_program_id = Keypair::new();

    // root
    let (root_pda, _root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[], &another_program_id.pubkey());
    let root_account = AccountSharedData::new(0, 0, &system_program);

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let (signing_pda, signing_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[SIGNING_SESSION_PDA], &PROGRAM_ID);
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountIncorrectOwner as u32,
        ))],
    );
}

#[test]
fn test_initialize_signing_session_not_writable_signing_session_account_failure() {
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

    // root
    let (
        root_pda,
        _root_pda_bump,
        root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let next_session_id = root_pda_init_state.last_id + 1; // this is because the next session id must be the incremented current one from root pda
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &next_session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new_readonly(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::SigningSessionAccountMustBeWritable as u32,
        ))],
    );
}

#[test]
fn test_initialize_signing_session_root_pda_not_initialized_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let _operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 0;

    // root
    let (root_pda, _root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_account = AccountSharedData::new(0, 0, &PROGRAM_ID);

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[&root_pda.to_bytes(), &[0u8], SIGNING_SESSION_PDA],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountMustBeInitialized as u32,
        ))],
    );
}

#[test]
fn test_initialize_signing_session_root_not_writable_failure() {
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

    // root
    let (
        root_pda,
        _root_pda_bump,
        root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let next_session_id = root_pda_init_state.last_id + 1;
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &next_session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountMustBeWrittable as u32,
        ))],
    );
}

#[test]
fn test_initialize_signing_session_payer_is_not_signer_failure() {
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

    // root
    let (
        root_pda,
        _root_pda_bump,
        root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let next_session_id = root_pda_init_state.last_id + 1; // this is because the next session id must be the incremented current one from root pda
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &next_session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(payer.into(), false),
            AccountMeta::new(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (payer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::PayerMustEqualSigner as u32,
        ))],
    );
}

#[test]
fn test_initialize_signing_session_signer_is_not_operator_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();

    let not_operator_signer = Keypair::new();
    let not_operator_signer_account =
        AccountSharedData::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

    let session_id = 0;

    // root
    let (
        root_pda,
        _root_pda_bump,
        root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing
    let next_session_id = root_pda_init_state.last_id + 1; // this is because the next session id must be the incremented current one from root pda
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &next_session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &system_program);

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(not_operator_signer.pubkey(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (
                not_operator_signer.pubkey(),
                not_operator_signer_account.clone().into(),
            ),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::SignerIsNotOperator as u32,
        ))],
    );
}

#[test]
fn test_re_initialize_signing_session_failure() {
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

    // root
    let (
        root_pda,
        _root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    let (signing_pda, signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![], // approvals
            SigningSessionPhase::Active,
            cpi_instruction_accounts.clone(),
            cpi_instruction_data.clone(),
        );

    let ix_data_initialize_signing_session = InitializeSigningSessionIxData {
        instruction_data: cpi_instruction_data.clone(),
        instruction_accounts: cpi_instruction_accounts.clone(),
        bump: signing_pda_bump,
    };
    let data_initialize_signing_session = [
        vec![ProgramIx::InitializeSigningSession as u8],
        to_vec(&ix_data_initialize_signing_session).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_initialize_signing_session,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::err(ProgramError::Custom(
                MosaicError::SigningSessionAccountMustNotBeInitialized as u32,
            )),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );
}
