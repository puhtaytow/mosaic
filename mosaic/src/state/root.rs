use crate::{
    errors::MosaicError, instructions::init_root::InitializeRootIxData, state::PackUnpack,
};
use pinocchio::{Address, error::ProgramError};

/// root data
#[derive(Clone, borsh::BorshDeserialize, borsh::BorshSerialize, Debug)]
pub struct Root {
    /// keys of multisig operators
    pub operators: Vec<Address>,

    /// last approval id
    pub last_id: u16,

    /// required operator approvals
    pub threshold: u8,

    /// program governed by multisig
    pub destination_program: Address,

    /// cannonical bump
    pub bump: u8,
}

impl Root {
    pub fn init(data: InitializeRootIxData) -> Self {
        Self {
            operators: data.operators,
            last_id: 0,
            threshold: data.threshold,
            destination_program: data.destination_program,
            bump: data.bump,
        }
    }
}

impl Root {
    /// returns signer index among known operators
    pub fn operator_index(&self, signer: &Address) -> Result<usize, ProgramError> {
        self.operators
            .iter()
            .position(|operator| operator == signer)
            .ok_or(MosaicError::SignerIsNotOperator.into())
    }

    /// check if destination program address match passed within instruction
    pub fn destination_program_address_must_match(
        &self,
        destination_program: &Address,
    ) -> Result<(), ProgramError> {
        (&self.destination_program == destination_program)
            .then_some(())
            .ok_or(MosaicError::ProvidedDestinationProgramMismatchWithRootDestinationProgram.into())
    }

    /// checks if signer is present among known operators
    pub fn signer_must_be_operator(&self, signer: &Address) -> Result<(), ProgramError> {
        self.operator_index(signer).map(|_| ())
    }

    /// increments last id session
    pub fn increment_last_id(&mut self) -> Result<(), ProgramError> {
        self.last_id = self
            .last_id
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(())
    }
}

impl PackUnpack for Root {}
