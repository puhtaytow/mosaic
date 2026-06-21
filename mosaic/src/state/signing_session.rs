use crate::{
    errors::MosaicError, instructions::init_signing_session::InitializeSigningSessionIxData,
    state::PackUnpack,
};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{Address, error::ProgramError};

/// proposal phase
#[derive(Clone, Copy, Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum SigningSessionPhase {
    Uninitialized,
    Active,
    Approved,
    Executed,
}

impl From<u8> for SigningSessionPhase {
    fn from(value: u8) -> Self {
        match value {
            0 => SigningSessionPhase::Uninitialized,
            1 => SigningSessionPhase::Active,
            2 => SigningSessionPhase::Approved,
            3 => SigningSessionPhase::Executed,
            _ => panic!("invalid account state value: {value}"),
        }
    }
}

impl From<SigningSessionPhase> for u8 {
    fn from(value: SigningSessionPhase) -> Self {
        match value {
            SigningSessionPhase::Uninitialized => 0,
            SigningSessionPhase::Active => 1,
            SigningSessionPhase::Approved => 2,
            SigningSessionPhase::Executed => 3,
        }
    }
}

/// easy to serialize repr of AccountView
#[derive(Clone, BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct InstructionAccount {
    pub pubkey: [u8; 32],
    pub signer: bool,
    pub writable: bool,
}

impl InstructionAccount {
    /// returns serialized data with length
    pub fn serialize(&self) -> Result<(Vec<u8>, usize), ProgramError> {
        let data = borsh::to_vec(&self).map_err(|_| ProgramError::InvalidAccountData)?;
        let size = data.len();
        Ok((data, size))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        borsh::from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

/// signing session data
#[derive(Clone, BorshDeserialize, BorshSerialize, Debug)]
pub struct SigningSession {
    /// proposal id
    pub session_id: u16,

    /// associated root pda // its not used for security checks but for account identification purposes
    pub root_pda: Address,

    /// current phase
    pub phase: SigningSessionPhase,

    /// bitmap of operators who signed, indexed by Root::operators order
    pub approvals_bitmap: u32,

    /// instruction data to execute after consensus being reached
    pub instruction_data: Vec<u8>,

    /// instruction accounts to instruction_data
    pub instruction_accounts: Vec<Vec<u8>>,

    /// cannonical bump
    pub bump: u8,
}

impl SigningSession {
    pub fn init(data: InitializeSigningSessionIxData, id: u16, root_pda: &Address) -> Self {
        Self {
            session_id: id,
            root_pda: *root_pda,
            phase: SigningSessionPhase::Active,
            approvals_bitmap: 0,
            instruction_data: data.instruction_data,
            instruction_accounts: data.instruction_accounts,
            bump: data.bump,
        }
    }
}

impl SigningSession {
    /// checks if amount of approvals reached expected threshold
    pub fn check_approvals_reaching_threshold(&self, threshold: usize) -> bool {
        self.approvals_bitmap.count_ones() as usize == threshold
    }

    /// progress signing phase with check
    pub fn progress_phase_checked(&mut self) -> Result<(), ProgramError> {
        (self.phase != SigningSessionPhase::Executed)
            .then_some(())
            .ok_or::<ProgramError>(MosaicError::SigningSessionPhaseAtFinalStage.into())?;
        self.phase = ((self.phase as u8)
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?)
        .into();
        Ok(())
    }

    /// marks signer approval with check if it already was casted
    pub fn approve_checked(&mut self, signer_index: usize) -> Result<(), ProgramError> {
        let signer_mask = 1_u32
            .checked_shl(signer_index as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        (self.approvals_bitmap & signer_mask == 0)
            .then_some(())
            .ok_or::<ProgramError>(MosaicError::SigningSessionSignerAlreadyApproved.into())?;

        self.approvals_bitmap |= signer_mask;
        Ok(())
    }

    /// checks if signing session is active
    pub fn must_be_active(&self) -> Result<(), ProgramError> {
        (self.phase == SigningSessionPhase::Active)
            .then_some(())
            .ok_or(MosaicError::SigningSessionPhaseIncorrect.into())
    }

    /// checks if signing session is approved
    pub fn must_be_approved(&self) -> Result<(), ProgramError> {
        (self.phase == SigningSessionPhase::Approved)
            .then_some(())
            .ok_or(MosaicError::SigningSessionPhaseIncorrect.into())
    }

    /// checks if signing session is executed
    pub fn must_be_executed(&self) -> Result<(), ProgramError> {
        (self.phase == SigningSessionPhase::Executed)
            .then_some(())
            .ok_or(MosaicError::SigningSessionPhaseIncorrect.into())
    }

    /// checks if root last id equals the session id
    pub fn sessions_must_equal(&self, root_last_id: u16) -> Result<(), ProgramError> {
        (self.session_id == root_last_id)
            .then_some(())
            .ok_or(MosaicError::SigningSessionIdMustEqualRootLastId.into())
    }
}

impl PackUnpack for SigningSession {}
