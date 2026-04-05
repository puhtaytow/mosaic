use serde::{Deserialize, Serialize};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use mosaic::state::{root::Root, signing_session::SigningSession};

#[derive(Deserialize)]
pub(crate) struct SessionSpecFile {
    #[serde(default)]
    pub(crate) data_encoding: DataEncoding,
    pub(crate) data: String,
    #[serde(default)]
    pub(crate) accounts: Vec<SessionSpecAccountFile>,
}

#[derive(Deserialize)]
pub(crate) struct SessionSpecAccountFile {
    pub(crate) pubkey: String,
    #[serde(default)]
    pub(crate) writable: bool,
    #[serde(default)]
    pub(crate) signer: bool,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DataEncoding {
    #[default]
    Hex,
    Base64,
    Utf8,
}

pub(crate) struct LoadedSessionSpec {
    pub(crate) instruction_data: Vec<u8>,
    pub(crate) instruction_accounts: Vec<Vec<u8>>,
}

pub(crate) struct LoadedRoot {
    pub(crate) pubkey: Pubkey,
    pub(crate) data: Root,
}

pub(crate) struct LoadedSession {
    pub(crate) pubkey: Pubkey,
    pub(crate) data: SigningSession,
}

pub(crate) struct ResolvedSession {
    pub(crate) root: LoadedRoot,
    pub(crate) session: LoadedSession,
}

#[derive(Clone, Serialize)]
pub(crate) struct InstructionAccountView {
    pub(crate) pubkey: String,
    pub(crate) writable: bool,
    pub(crate) signer: bool,
}

pub(crate) struct ExecuteBuild {
    pub(crate) instruction: Instruction,
    pub(crate) remaining_accounts: Vec<InstructionAccountView>,
    pub(crate) required_outer_signers: Vec<Pubkey>,
    pub(crate) destination_program: Pubkey,
}

pub(crate) struct ExecutePlan {
    pub(crate) root_writable: bool,
    pub(crate) account_metas: Vec<solana_instruction::AccountMeta>,
    pub(crate) remaining_accounts: Vec<InstructionAccountView>,
    pub(crate) required_outer_signers: Vec<Pubkey>,
}
