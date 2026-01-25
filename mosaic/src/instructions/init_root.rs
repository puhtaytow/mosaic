use crate::{
    ID, ROOT_PDA,
    errors::MosaicError,
    instructions::root_pda_check,
    state::{PackUnpack, root::Root},
};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};

/// Initialize Operators
///
/// ### accounts:
///   0. `[WRITE, SIGNER]`
///   1. `[WRITE]` root pda
pub struct InitializeRootIxAccounts<'info> {
    pub payer: &'info AccountView,
    pub root: &'info AccountView,
}

impl<'info> TryFrom<&'info [AccountView]> for InitializeRootIxAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        // perform accounts attribute check
        let [payer, root, _system] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !payer.is_signer() {
            return Err(MosaicError::PayerMustEqualSigner.into());
        }
        if !root.is_writable() {
            return Err(MosaicError::RootAccountMustBeWrittable.into());
        }
        if !root.is_data_empty() {
            return Err(MosaicError::RootAccountMustNotBeInitialized.into());
        }

        Ok(Self { payer, root })
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct InitializeRootIxData {
    pub operators: Vec<Address>,
    pub threshold: u8,
    pub destination_program: Address,
    pub bump: u8,
}

impl<'info> TryFrom<&'info [u8]> for InitializeRootIxData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        Ok(borsh::from_slice::<Self>(&data).map_err(|_| ProgramError::InvalidInstructionData)?)
    }
}

pub struct InitializeOperators<'info> {
    pub accounts: InitializeRootIxAccounts<'info>,
    pub instruction_data: InitializeRootIxData,
}

impl<'info> TryFrom<(&'info [AccountView], &'info [u8])> for InitializeOperators<'info> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'info [AccountView], &'info [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeRootIxAccounts::try_from(accounts)?;
        let instruction_data = InitializeRootIxData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> InitializeOperators<'info> {
    pub fn handler(&mut self) -> ProgramResult {
        Self::mandatory_checks(&self.instruction_data, &self.accounts)?;

        let root_ix_data_bump = [self.instruction_data.bump];
        let root_seeds = [Seed::from(ROOT_PDA), Seed::from(&root_ix_data_bump)];
        let cpi_signer = Signer::from(&root_seeds);

        let (root_data, root_data_len) = Root::init(self.instruction_data.clone()).pack()?;

        // create account
        pinocchio_system::instructions::CreateAccount {
            from: self.accounts.payer,
            to: self.accounts.root,
            space: root_data_len as u64,
            lamports: Rent::get()?.try_minimum_balance(root_data_len)?,
            owner: &ID.into(),
        }
        .invoke_signed(&[cpi_signer])?;

        // write to account
        let mut root_account = self.accounts.root.try_borrow_mut()?;
        root_account[..root_data.len()].copy_from_slice(&root_data);

        Ok(())
    }

    #[must_use]
    fn mandatory_checks(
        ix_data: &InitializeRootIxData,
        accounts: &InitializeRootIxAccounts,
    ) -> Result<(), ProgramError> {
        root_pda_check(&accounts.root.address(), &[ix_data.bump])?;

        if ix_data.operators.is_empty() {
            return Err(MosaicError::OperatorsCountMustBePositive.into());
        }

        if ix_data.operators.len() > 20 {
            return Err(MosaicError::ReachedOperatorsLimit.into());
        }

        if ix_data.threshold == 0 {
            return Err(MosaicError::ThresholdCanNotBeZero.into());
        }

        if ix_data.threshold as usize > ix_data.operators.len() {
            return Err(MosaicError::ThresholdCanNotBeHigherThanLenOfOperators.into());
        }

        Ok(())
    }
}
