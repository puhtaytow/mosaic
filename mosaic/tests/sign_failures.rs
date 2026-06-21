mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    errors::MosaicError,
    instructions::{Instruction as ProgramIx, sign::SignIxData},
    state::signing_session::{SigningSession, SigningSessionPhase},
};

use solana_program::example_mocks::{solana_keypair::Keypair, solana_signer::Signer};
use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[test]
fn test_sign_payer_is_not_signer_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

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

    // signing session
    let (signing_pda, signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            0, // approvals bitmap
            SigningSessionPhase::Active,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // sign
    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
        vec![
            AccountMeta::new(payer.into(), false),
            AccountMeta::new_readonly(root_pda, false),
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
fn test_sign_last_wrap_session() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

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
    let (signing_pda, signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            approvals_bitmap(&operators_pubkey, &[operators_pubkey[1]]),
            SigningSessionPhase::Active,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // sign
    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();
    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
            AccountMeta::new_readonly(system_program, false),
        ],
    );
    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[
            Check::success(),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );

    let updated_data = result.get_account(&signing_pda).unwrap();
    let parsed_data = borsh::from_slice::<SigningSession>(&updated_data.data).unwrap();

    assert!(parsed_data.phase == SigningSessionPhase::Approved);
    assert!(has_approval(
        &operators_pubkey,
        parsed_data.approvals_bitmap,
        signer
    ));
    assert!(has_approval(
        &operators_pubkey,
        parsed_data.approvals_bitmap,
        operators_pubkey[1]
    ));
    assert!(parsed_data.bump == signing_pda_bump)
}

#[test]
fn test_sign_twice_same_signer_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

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
    let (signing_pda, signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            approvals_bitmap(&operators_pubkey, &[signer]),
            SigningSessionPhase::Active,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // sign
    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
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
        &[
            Check::err(ProgramError::Custom(
                MosaicError::SigningSessionSignerAlreadyApproved as u32,
            )),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );
}

#[test]
fn test_sign_signer_is_not_operator_failure() {
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

    // signing session
    let (signing_pda, signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            0, // approvals bitmap
            SigningSessionPhase::Active,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    // sign
    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
        vec![
            AccountMeta::new(not_operator_signer.pubkey(), true),
            AccountMeta::new_readonly(root_pda, false),
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
        &[
            Check::err(ProgramError::Custom(
                MosaicError::SignerIsNotOperator as u32,
            )),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );
}

#[test]
fn test_sign_root_incorrect_owner_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;
    let root_pda = Pubkey::new_unique();
    let root_account = AccountSharedData::new(1000000, 100, &system_program); // niewłaściwy owner

    // storage
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
            0,
            SigningSessionPhase::Active,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
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
            (root_pda, root_account.into()),
            (signing_pda, signing_account.clone().into()),
            (system_program, system_account.clone()),
        ],
        &[Check::err(ProgramError::Custom(
            MosaicError::RootAccountIncorrectOwner as u32,
        ))],
    );
}

#[test]
fn test_sign_signing_session_not_writable_failure() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);
    let (system_program, system_account) = mollusk_svm::program::keyed_account_for_system_program();

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

    // storage
    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);

    // record program accounts and instruction data
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    let (signing_pda, signing_pda_bump, _, signing_account) = prepare_signing_session(
        &mollusk,
        session_id,
        root_pda,
        0,
        SigningSessionPhase::Active,
        cpi_instruction_accounts,
        cpi_instruction_data,
    );

    let ix_data_sign = SignIxData {
        bump: signing_pda_bump,
    };
    let data_sign = [vec![ProgramIx::Sign as u8], to_vec(&ix_data_sign).unwrap()].concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data_sign,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new_readonly(signing_pda, false), // ← NIE WRITABLE
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
