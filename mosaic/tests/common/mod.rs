#![allow(dead_code)]

use mollusk_svm::Mollusk;

use mosaic::{
    ID, ROOT_PDA, SIGNING_SESSION_PDA,
    state::{
        root::Root,
        signing_session::{InstructionAccount, SigningSession, SigningSessionPhase},
    },
};

use solana_sdk::{account::AccountSharedData, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(ID);
pub const DESTINATION_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("5Dzce8Ww59s3G4WeX9y3gjFJP2ZZNzKTVoXRmwhnZjS4");
pub const _DEFAULT_LOADER_KEY: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

pub const MOSAIC_BINARY_PATH: &str = "./target/deploy/mosaic";
pub const EXAMPLE_BINARY_PATH: &str = "./target/deploy/mosaic";

#[derive(Debug, PartialEq, Clone)]
pub struct Operators {
    pub operators: Vec<(Pubkey, AccountSharedData)>,
    pub threshold: u8,
}

impl Operators {
    /// returns operators with funded accounts and 2/3 threshold
    pub fn new(num: u8, owner: Pubkey) -> Self {
        let mut operators = vec![];
        for _ in 0..num {
            operators.push((
                Pubkey::new_unique(),
                AccountSharedData::new(1 * LAMPORTS_PER_SOL, 0, &owner),
            ));
        }
        let threshold = (operators.len() as u8 * 2) / 3;
        Self {
            operators,
            threshold,
        }
    }
}

/// prepares fixture with state account for tests of arbitrary accounts execution
pub fn prepare_state_for_arbitrary(
    mollusk: &Mollusk,
    operators: Operators,
    operators_pubkey: Vec<Pubkey>,
    session_id: u16,
) -> (Pubkey, u8, Root, Vec<u8>, AccountSharedData) {
    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_pda_init_state = Root {
        operators: operators_pubkey,
        last_id: session_id,
        destination_program: DESTINATION_PROGRAM_ID,
        threshold: operators.threshold,
        bump: root_pda_bump,
    };
    let root_pda_initial_state_serialized = borsh::to_vec(&root_pda_init_state).unwrap();
    let root_pda_size = root_pda_initial_state_serialized.len();
    let root_pda_rent = mollusk.sysvars.rent.minimum_balance(root_pda_size);
    let mut root_account = AccountSharedData::new(root_pda_rent, root_pda_size, &PROGRAM_ID);
    root_account.set_data_from_slice(&root_pda_initial_state_serialized.clone());

    (
        root_pda,
        root_pda_bump,
        root_pda_init_state,
        root_pda_initial_state_serialized,
        root_account,
    )
}

/// prepares fixture with root account state for tests
pub fn prepare_root(
    mollusk: &Mollusk,
    operators: Operators,
    operators_pubkey: Vec<Pubkey>,
    session_id: u16,
    destination_program: Pubkey,
) -> (Pubkey, u8, Root, Vec<u8>, AccountSharedData) {
    let (root_pda, root_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[ROOT_PDA], &PROGRAM_ID);
    let root_pda_init_state = Root {
        operators: operators_pubkey,
        last_id: session_id,
        destination_program,
        threshold: operators.threshold,
        bump: root_pda_bump,
    };
    let root_pda_initial_state_serialized = borsh::to_vec(&root_pda_init_state).unwrap();
    let root_pda_size = root_pda_initial_state_serialized.len();
    let root_pda_rent = mollusk.sysvars.rent.minimum_balance(root_pda_size);
    let mut root_account = AccountSharedData::new(root_pda_rent, root_pda_size, &PROGRAM_ID);
    root_account.set_data_from_slice(&root_pda_initial_state_serialized.clone());

    (
        root_pda,
        root_pda_bump,
        root_pda_init_state,
        root_pda_initial_state_serialized,
        root_account,
    )
}

/// prepares fixture with signing session account state for tests
pub fn prepare_signing_session(
    mollusk: &Mollusk,
    session_id: u16,
    root_pda: Pubkey,
    approvals: Vec<Pubkey>,
    phase: SigningSessionPhase,
    cpi_instruction_accounts: Vec<Vec<u8>>,
    cpi_instruction_data: Vec<u8>,
) -> (Pubkey, u8, Vec<u8>, AccountSharedData) {
    let (signing_pda, signing_pda_bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            &root_pda.to_bytes(),
            &session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
        ],
        &PROGRAM_ID,
    );
    let signing_init_state = SigningSession {
        session_id,
        root_pda,
        phase,
        approvals,
        instruction_data: cpi_instruction_data,
        instruction_accounts: cpi_instruction_accounts,
        bump: signing_pda_bump,
    };
    let signing_init_state_serialized = borsh::to_vec(&signing_init_state).unwrap();
    let signing_size = signing_init_state_serialized.len();
    let signing_init_state_rent = mollusk.sysvars.rent.minimum_balance(signing_size);
    let mut signing_account =
        AccountSharedData::new(signing_init_state_rent, signing_size, &PROGRAM_ID);
    signing_account.set_data_from_slice(&signing_init_state_serialized.clone());

    (
        signing_pda,
        signing_pda_bump,
        signing_init_state_serialized,
        signing_account,
    )
}

/// prepares records program data
pub fn records_program_ix_accs(storage_id: Pubkey, authority: Pubkey) -> (Vec<Vec<u8>>, Vec<u8>) {
    // record program accounts and instruction data
    let mut cpi_instruction_accounts = vec![];

    // the accounts the destination program CPI needs
    // order does matter
    cpi_instruction_accounts.push(
        InstructionAccount {
            pubkey: storage_id.to_bytes(),
            signer: false,
            writable: true,
        }
        .serialize()
        .unwrap()
        .0,
    );
    cpi_instruction_accounts.push(
        InstructionAccount {
            pubkey: authority.to_bytes(),
            signer: true, // signer in the CPI
            writable: false,
        }
        .serialize()
        .unwrap()
        .0,
    );

    let offset = 0u64;
    let data_to_write_after_33 = &[0x2A]; // 42
    let data_length = data_to_write_after_33.len() as u32;

    let mut cpi_instruction_data = Vec::new();
    cpi_instruction_data.push(1); // ix discriminator
    cpi_instruction_data.extend_from_slice(&offset.to_le_bytes());
    cpi_instruction_data.extend_from_slice(&data_length.to_le_bytes()); // 4 bytes
    cpi_instruction_data.extend_from_slice(data_to_write_after_33);

    (cpi_instruction_accounts, cpi_instruction_data)
}

pub fn prepare_storage_account(
    mollusk: &Mollusk,
    session_id: u16,
    root_pda: Pubkey,
) -> (Pubkey, AccountSharedData) {
    let (storage_pda, _storage_pda_bump) =
        solana_sdk::pubkey::Pubkey::find_program_address(&[], &DESTINATION_PROGRAM_ID);

    let mut storage_header = vec![];
    storage_header.extend_from_slice(&[session_id as u8]);
    storage_header.extend_from_slice(&root_pda.to_bytes());

    // calculate total size needed
    let header_size = storage_header.len();
    // 33 bytes / version 1 byte + 32 bytes pubkey
    let data_size = 1;
    // 1 byte for our data
    let total_storage_size = header_size + data_size;

    let storage_pda_rent = mollusk.sysvars.rent.minimum_balance(total_storage_size);
    let mut storage_pda_account = AccountSharedData::new(
        storage_pda_rent,
        total_storage_size,
        &DESTINATION_PROGRAM_ID,
    );

    let mut storage_data = vec![0u8; total_storage_size];
    storage_data[..header_size].copy_from_slice(&storage_header);
    storage_pda_account.set_data_from_slice(&storage_data);
    (storage_pda, storage_pda_account)
}
