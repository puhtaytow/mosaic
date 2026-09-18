mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    instructions::{Instruction as ProgramIx, execute::ExecuteIxData},
    state::signing_session::{SigningSession, SigningSessionPhase},
};

use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
};

#[test]
fn test_execute() {
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
    let (storage_pda, storage_pda_account) =
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
            approvals_bitmap(&operators_pubkey, &[signer, operators_pubkey[1]]),
            SigningSessionPhase::Approved, // signing session phase / must be Approved to Execute
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

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
            AccountMeta::new(signer.into(), true),      // 0: payer/signer
            AccountMeta::new_readonly(root_pda, false), // 1: root_pda
            AccountMeta::new(signing_pda, false),       // 2: signing_pda
            AccountMeta::new_readonly(system_program, false), // 3: system_program
            AccountMeta::new_readonly(DESTINATION_PROGRAM_ID, false), // 4: DESTINATION_PROGRAM_ID
            AccountMeta::new(storage_pda, false),       // 5: storage_pda (remaining[0])
        ],
    );

    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()), // 0: payer/signer
            (root_pda, root_account.clone().into()),        // 1: root_pda
            (signing_pda, signing_account.clone().into()),  // 2: signing_pda
            (system_program, system_account.clone().into()), // 3: system_program
            (DESTINATION_PROGRAM_ID, dst_program_account.clone().into()), // 4: DESTINATION_PROGRAM_ID
            (storage_pda, storage_pda_account.clone().into()), // 5: storage_pda (remaining[0])
        ],
        &[
            Check::success(),
            Check::account(&signing_pda).owner(&PROGRAM_ID).build(),
        ],
    );

    let updated_signing_session_pda_account = result.get_account(&signing_pda).unwrap();
    let parsed_signing_session_pda_data =
        borsh::from_slice::<SigningSession>(&updated_signing_session_pda_account.data).unwrap();

    let copy_of_initial_storage_data = &storage_pda_account.clone().into();
    let updated_storage_pda_account = result.get_account(&storage_pda).unwrap();

    assert!(updated_storage_pda_account != copy_of_initial_storage_data,);
    assert!(parsed_signing_session_pda_data.phase == SigningSessionPhase::Executed);
    assert!(has_approval(
        &operators_pubkey,
        parsed_signing_session_pda_data.approvals_bitmap,
        signer
    ));
    assert!(has_approval(
        &operators_pubkey,
        parsed_signing_session_pda_data.approvals_bitmap,
        operators_pubkey[1]
    ));
    assert!(parsed_signing_session_pda_data.bump == signing_pda_bump)
}
