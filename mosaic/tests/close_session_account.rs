mod common;

use {
    borsh::to_vec,
    common::*,
    mollusk_svm::{Mollusk, result::Check},
};

use mosaic::{
    instructions::{Instruction as ProgramIx, close_session_account::CloseSessionAccountIxData},
    state::signing_session::SigningSessionPhase,
};

use solana_sdk::{
    account::ReadableAccount,
    instruction::{AccountMeta, Instruction},
};

#[test]
fn test_close_session_account() {
    let mollusk = Mollusk::new(&PROGRAM_ID, MOSAIC_BINARY_PATH);

    let (system_program, _) = mollusk_svm::program::keyed_account_for_system_program();

    let operators = Operators::new(3, system_program);
    let operators_pubkey: Vec<_> = operators
        .operators
        .iter()
        .map(|operator| operator.0)
        .collect();
    let (signer, signer_account) = operators.operators[0].clone();

    let session_id = 1;

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

    let (storage_pda, _storage_pda_account) =
        prepare_storage_account(&mollusk, session_id, root_pda);
    let (cpi_instruction_accounts, cpi_instruction_data) =
        records_program_ix_accs(storage_pda, root_pda);

    let (signing_pda, _signing_pda_bump, _signing_init_state_serialized, signing_account) =
        prepare_signing_session(
            &mollusk,
            session_id,
            root_pda,
            vec![signer, operators_pubkey[1]],
            SigningSessionPhase::Executed,
            cpi_instruction_accounts,
            cpi_instruction_data,
        );

    let payer_initial_lamports = signer_account.lamports();
    let signing_initial_lamports = signing_account.lamports();

    let ix_data = CloseSessionAccountIxData {};
    let data = [
        vec![ProgramIx::CloseSessionAccount as u8],
        to_vec(&ix_data).unwrap(),
    ]
    .concat();

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new(signer.into(), true),
            AccountMeta::new_readonly(root_pda, false),
            AccountMeta::new(signing_pda, false),
        ],
    );

    let result: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (signer.into(), signer_account.clone().into()),
            (root_pda, root_account.clone().into()),
            (signing_pda, signing_account.clone().into()),
        ],
        &[Check::success()],
    );

    let updated_payer = result.get_account(&signer).unwrap();
    let updated_signing = result.get_account(&signing_pda).unwrap();

    assert_eq!(
        updated_payer.lamports,
        payer_initial_lamports + signing_initial_lamports
    );
    assert_eq!(updated_signing.lamports, 0);
    assert!(updated_signing.data.is_empty());
}
