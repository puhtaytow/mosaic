use pinocchio::{Address, error::ProgramError};

use crate::{
    ID, {ROOT_PDA, SIGNING_SESSION_PDA},
};

pub mod close_session_account;
pub mod execute;
pub mod init_root;
pub mod init_signing_session;
pub mod sign;

#[repr(u8)]
pub enum Instruction {
    InitializeOperators,
    InitializeSigningSession,
    Sign,
    Execute,
    CloseSessionAccount,
}

impl TryFrom<&u8> for Instruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(Instruction::InitializeOperators),
            1 => Ok(Instruction::InitializeSigningSession),
            2 => Ok(Instruction::Sign),
            3 => Ok(Instruction::Execute),
            4 => Ok(Instruction::CloseSessionAccount),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Checks that the signing session address and bump are canonical.
pub fn signing_session_pda_check(
    key: &Address,
    root_pda: &[u8],
    session_id: u16,
    bump: &[u8],
) -> Result<(), ProgramError> {
    check_pda(
        key,
        &[root_pda, &session_id.to_be_bytes(), SIGNING_SESSION_PDA],
        bump,
    )
}

/// Checks that the root address and bump are canonical for this deployment.
pub fn root_pda_check(key: &Address, bump: &[u8]) -> Result<(), ProgramError> {
    check_pda(key, &[ROOT_PDA], bump)
}

/// Checks the canonical PDA using seeds without the separately supplied bump.
fn check_pda(key: &Address, seeds: &[&[u8]], bump: &[u8]) -> Result<(), ProgramError> {
    let (canonical_pda, canonical_bump) =
        Address::try_find_program_address(seeds, &ID.into()).ok_or(ProgramError::InvalidSeeds)?;
    if bump != [canonical_bump] {
        return Err(ProgramError::InvalidSeeds);
    }
    if key != &canonical_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Allows for dynamic dispatch with invoke signed; up to 20 arbitrary accounts
#[macro_export]
macro_rules! invoke_signed_dynamic {
    ($instruction:expr, $account_infos:expr, $signers:expr) => {{
        macro_rules! try_invoke {
            ($n:expr) => {
                if $account_infos.len() == $n {
                    let arr: &[&pinocchio::AccountView; $n] =
                        $account_infos.as_slice().try_into().unwrap();
                    break invoke_signed($instruction, arr, $signers);
                }
            };
        }

        loop {
            try_invoke!(1);
            try_invoke!(2);
            try_invoke!(3);
            try_invoke!(4);
            try_invoke!(5);
            try_invoke!(6);
            try_invoke!(7);
            try_invoke!(8);
            try_invoke!(9);
            try_invoke!(10);
            try_invoke!(11);
            try_invoke!(12);
            try_invoke!(13);
            try_invoke!(14);
            try_invoke!(15);
            try_invoke!(16);
            try_invoke!(17);
            try_invoke!(18);
            try_invoke!(19);
            try_invoke!(20);
            break Err(ProgramError::InvalidArgument.into());
        }
    }};
}
