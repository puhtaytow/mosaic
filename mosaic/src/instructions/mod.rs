use pinocchio::{Address, error::ProgramError};

use crate::{
    ID, {ROOT_PDA, SIGNING_SESSION_PDA},
};

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
}

impl TryFrom<&u8> for Instruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(Instruction::InitializeOperators),
            1 => Ok(Instruction::InitializeSigningSession),
            2 => Ok(Instruction::Sign),
            3 => Ok(Instruction::Execute),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Checks if the provided signing session PDA is correct
pub fn signing_session_pda_check(
    key: &Address,
    root_pda: &[u8],
    session_id: u16,
    bump: &[u8],
) -> Result<(), ProgramError> {
    check_pda(
        key,
        &[
            root_pda,
            &session_id.to_be_bytes(),
            SIGNING_SESSION_PDA,
            bump,
        ],
    )
}

/// Checks if the provided root PDA is correct
pub fn root_pda_check(key: &Address, bump: &[u8]) -> Result<(), ProgramError> {
    check_pda(key, &[ROOT_PDA, bump])
}

fn check_pda(key: &Address, seeds: &[&[u8]]) -> Result<(), ProgramError> {
    let found_pda = Address::create_program_address(seeds, &ID.into())
        .map_err(|_| ProgramError::InvalidSeeds)?;
    if key != &found_pda {
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
