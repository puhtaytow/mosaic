mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    SIGNING_SESSION_PDA,
    errors::MosaicError,
    instructions::{Instruction as ProgramIx, execute::ExecuteIxData},
    state::signing_session::{SigningSession, SigningSessionPhase},
};

use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[test]
fn test_execute_payer_is_not_signer_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (payer, signer_account) = operators.operators[0].clone();

    // used in root pda as last_id and in signing session as id
    let session_id = 1;

    // root
    let (
        root_pda,
        __root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing session
    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![payer, operators_pubkey[1]], // approvals
            SigningSessionPhase::Approved,    // signing session phase / must be Approved to Execute
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // storage
    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // execute
    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(payer.into(), false),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (payer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (storage_pda, storage_pda_account.clone().into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::PayerMustEqualSigner as u32,
        ))],
    );
}

#[test]
fn test_re_execute_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    // used in root pda as last_id and in signing session as id
    let session_id = 1;

    // root
    let (
        root_pda,
        __root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing session
    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![signer, operators_pubkey[1]], // approvals
            SigningSessionPhase::Executed,     // signing session phase
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // storage
    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // execute
    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (storage_pda, storage_pda_account.clone().into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::err(ProgramError::Custom(
                MosaicError::SigningSessionPhaseIncorrect as u32,
            )),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );

    let updated_signing_session_pda_account = result.get_account(&signing_pda).unwrap();
    let _parsed_signing_session_pda_data =
        borsh::from_slice::<SigningSession>(&updated_signing_session_pda_account.data).unwrap();

    let copy_of_initial_storage_data = &storage_pda_account.clone().into();
    let updated_storage_pda_account = result.get_account(&storage_pda).unwrap();

    assert!(updated_storage_pda_account == copy_of_initial_storage_data,);
}

#[test]
fn test_execute_active_phase_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    // used in root pda as last_id and in signing session as id
    let session_id = 1;

    // root
    let (
        root_pda,
        __root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing session
    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![signer, operators_pubkey[1]], // approvals
            SigningSessionPhase::Active,       // signing session phase
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // storage
    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // execute
    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (storage_pda, storage_pda_account.clone().into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::err(ProgramError::Custom(
                MosaicError::SigningSessionPhaseIncorrect as u32,
            )),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );

    let updated_signing_session_pda_account = result.get_account(&signing_pda).unwrap();
    let _parsed_signing_session_pda_data =
        borsh::from_slice::<SigningSession>(&updated_signing_session_pda_account.data).unwrap();

    let copy_of_initial_storage_data = &storage_pda_account.clone().into();
    let updated_storage_pda_account = result.get_account(&storage_pda).unwrap();

    assert!(updated_storage_pda_account == copy_of_initial_storage_data,);
}

#[test]
fn test_execute_signing_session_not_writable_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &_DEFAULT_LOADER_KEY);

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

    let (root_pda, _, _, _, root_account) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    let (signing_pda, _, _, signing_account) = prepare_signing_session(
        &mollusk,
        session_id,
        root_pda,
        vec![signer, operators_pubkey[1]],
        SigningSessionPhase::Approved,
        cpi_instruction_accounts,
        cpi_instruction_data,
    );

    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new_readonly(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );

    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.into()),
            (root_pda, root_account.into()),
            (signing_pda, signing_account.into()),
            (storage_pda, storage_pda_account.into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.into()),
            (system_program, system_account),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::SigningSessionAccountMustBeWritable as u32,
        ))],
    );
}

#[test]
fn test_execute_root_incorrect_owner_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

    // Root
    // incorrect owner
    let root_pda = Pubkey::new_unique();
    let root_account = AccountSharedData::new(0, 100, &system_program);

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    let (signing_pda, _, _, signing_account) = prepare_signing_session(
        &mollusk,
        session_id,
        root_pda,
        vec![signer, operators_pubkey[1]],
        SigningSessionPhase::Approved,
        cpi_instruction_accounts,
        cpi_instruction_data,
    );

    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );

    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.into()),
            (root_pda, root_account.into()),
            (signing_pda, signing_account.into()),
            (storage_pda, storage_pda_account.into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.into()),
            (system_program, system_account),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountIncorrectOwner as u32,
        ))],
    );
}

#[test]
fn test_execute_signing_session_incorrect_owner_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

    let (root_pda, _, _, _, root_account) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // Signing
    let signing_pda = Pubkey::new_unique();
    // incorrect owner
    let mut signing_account = AccountSharedData::new(0, 200, &system_program);

    let signing_init_state = SigningSession {
        session_id,
        root_pda: root_pda,
        phase: SigningSessionPhase::Approved,
        approvals: vec![signer, operators_pubkey[1]],
        instruction_data: vec![],
        instruction_accounts: vec![],
        bump: 0,
    };
    let signing_data = borsh::to_vec(&signing_init_state).unwrap();
    signing_account.set_data_from_slice(&signing_data);

    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );

    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.into()),
            (root_pda, root_account.into()),
            (signing_pda, signing_account.into()),
            (storage_pda, storage_pda_account.into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.into()),
            (system_program, system_account),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::SigningSessionAccountIncorrectOwner as u32,
        ))],
    );
}

#[test]
fn test_execute_signing_session_not_initialized_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

    let (root_pda, _, _, _, root_account) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey,
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // Signing
    let (signing_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_account = AccountSharedData::new(0, 0, &PROGRAM_ID);

    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );

    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.into()),
            (root_pda, root_account.into()),
            (signing_pda, signing_account.into()),
            (storage_pda, storage_pda_account.into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.into()),
            (system_program, system_account),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::SigningSessionAccountMustBeInitialized as u32,
        ))],
    );
}

#[test]
fn test_execute_but_session_id_does_not_equal_root_last_id_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();
    let dst_program_account = AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    // used in root pda as last_id and in signing session as id
    let session_id = 0;

    // root
    let (
        root_pda,
        __root_pda_bump,
        _root_pda_init_state,
        _root_pda_initial_state_serialized,
        root_account,
    ) = prepare_root(
        &mollusk,
        operators,
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing session
    let wrong_session_id_does_not_equal_root = session_id + 9;
    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            wrong_session_id_does_not_equal_root,
            root_pda,
            vec![signer, operators_pubkey[1]], // approvals
            SigningSessionPhase::Approved, // signing session phase / must be Approved to Execute
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // storage
    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // execute
    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new(storage_pda, false),
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (storage_pda, storage_pda_account.clone().into()),
            (DESTINATION_PROGRAM_ID, dst_program_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::InvalidSeeds)],
    );
}

#[test]
fn test_execute_destination_program_mismatch_failure() {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    mollusk.add_program(&DESTINATION_PROGRAM_ID, "tests/spl_record");

    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

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
        operators_pubkey.clone(),
        session_id,
        DESTINATION_PROGRAM_ID.as_ref().try_into().unwrap(),
    );

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    // signing session
    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![signer, operators_pubkey[1]],
            SigningSessionPhase::Approved,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // storage
    let (storage_pda, storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // wrong destination program
    let wrong_destination_program = Pubkey::new_unique();
    let wrong_dst_program_account =
        AccountSharedData::new(0, 0, &solana_sdk::bpf_loader::id());

    let ix_data_execute = ExecuteIxData {};
    let data_execute = [
        vec![ProgramIx::Execute as u8],
        to_vec(&ix_data_execute).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_execute,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(wrong_destination_program, false),
            AccountMeta::new(storage_pda, false),
        ],
    );

    let _result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone().into()),
            (wrong_destination_program, wrong_dst_program_account.into()),
            (storage_pda, storage_pda_account.clone().into()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::ProvidedDestinationProgramMismatchWithRootDestinationProgram as u32,
        ))],
    );
}

