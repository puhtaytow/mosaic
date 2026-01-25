mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    SIGNING_SESSION_PDA,
    instructions::{
        Instruction as ProgramIx, init_signing_session::InitializeSigningSessionIxData,
    },
    state::{
        root::Root,
        signing_session::{SigningSession, SigningSessionPhase},
    },
};

use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
};

#[test]
fn test_initialize_signing_session() {
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
            AccountMeta::new(signer.into(), true),
            AccountMeta::new(root_pda, false),
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

    let updated_root_pda_account = result.get_account(&root_pda).unwrap();
    let parsed_root_pda_data = borsh::from_slice::<Root>(&updated_root_pda_account.data).unwrap();

    assert!(parsed_root_pda_data.last_id == next_session_id);
    assert!(parsed_root_pda_data.last_id == parsed_signing_session_pda_data.session_id);
    assert!(parsed_signing_session_pda_data.root_pda == root_pda);
    assert!(parsed_signing_session_pda_data.phase == SigningSessionPhase::Active);
    assert!(parsed_signing_session_pda_data.approvals.is_empty());
    assert!(parsed_signing_session_pda_data.instruction_data == cpi_instruction_data);
    assert!(parsed_signing_session_pda_data.instruction_accounts == cpi_instruction_accounts);
    assert!(parsed_signing_session_pda_data.bump == signing_pda_bump)
}
