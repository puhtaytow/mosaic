mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    instructions::{Instruction as ProgramIx, sign::SignIxData},
    state::signing_session::{SigningSession, SigningSessionPhase},
};

use solana_sdk::instruction::{AccountMeta, Instruction};

#[test]
fn test_sign() {
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

    let updated_signing_session_pda_account = result.get_account(&signing_pda).unwrap();
    let parsed_signing_session_pda_data =
        borsh::from_slice::<SigningSession>(&updated_signing_session_pda_account.data).unwrap();

    assert!(parsed_signing_session_pda_data.session_id == session_id);
    assert!(parsed_signing_session_pda_data.root_pda == root_pda);
    assert!(parsed_signing_session_pda_data.phase == SigningSessionPhase::Active);
    assert!(has_approval(
        &operators_pubkey,
        parsed_signing_session_pda_data.approvals_bitmap,
        signer
    ));
    assert!(parsed_signing_session_pda_data.bump == signing_pda_bump)
}
