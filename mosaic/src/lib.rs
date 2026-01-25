pub mod errors;
pub mod instructions;
pub mod security;
pub mod state;

use crate::instructions::{
    Instruction, execute::Execute, init_root::InitializeOperators,
    init_signing_session::InitializeSigningSession, sign::Sign,
};
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError, program_entrypoint};
use solana_program::{custom_heap_default, custom_panic_default};

custom_heap_default!();
custom_panic_default!();
program_entrypoint!(process_instruction);
// change the id accordingly
pinocchio_pubkey::declare_id!("s75D2Kb5WnVBsFQiSLj5E4oRgwDJU63487cSnp2khXh");

/// seed of the root PDA.
pub const ROOT_PDA: &[u8] = b"root_pda";
/// seed of the signing session PDA.
pub const SIGNING_SESSION_PDA: &[u8] = b"signing_session_pda";

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    mandatory_checks(program_id)?;

    let (opcode, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match Instruction::try_from(opcode)? {
        Instruction::InitializeOperators => {
            InitializeOperators::try_from((accounts, data))?.handler()
        }
        Instruction::InitializeSigningSession => {
            InitializeSigningSession::try_from((accounts, data))?.handler()
        }
        Instruction::Sign => Sign::try_from((accounts, data))?.handler(),
        Instruction::Execute => Execute::try_from((accounts, data))?.handler(),
    }
}

#[must_use]
fn mandatory_checks(program_id: &Address) -> Result<(), ProgramError> {
    if program_id != &crate::ID.into() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
