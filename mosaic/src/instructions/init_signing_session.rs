use crate::{
    ID, SIGNING_SESSION_PDA,
    errors::MosaicError,
    instructions::{root_pda_check, signing_session_pda_check},
    state::{PackUnpack, root::Root, signing_session::SigningSession},
};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};

/// Initialize Signing Session
///
/// ### accounts:
///   0. `[WRITE, SIGNER]`
///   1. `[WRITE]`  root pda
///   2. `[WRITE]`  signing session pda
pub struct InitializeSigningSessionIxAccounts<'info> {
    pub payer: &'info AccountView,
    pub root: &'info AccountView,
    pub signing_session: &'info AccountView,
}

impl<'info> TryFrom<&'info [AccountView]> for InitializeSigningSessionIxAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        // perform accounts attribute check
        let [payer, root, signing_session, _system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !root.owned_by(&ID.into()) {
            return Err(MosaicError::RootAccountIncorrectOwner.into());
        }
        if !payer.is_signer() {
            return Err(MosaicError::PayerMustEqualSigner.into());
        }
        if !root.is_writable() {
            return Err(MosaicError::RootAccountMustBeWrittable.into());
        }
        if root.is_data_empty() {
            return Err(MosaicError::RootAccountMustBeInitialized.into());
        }
        if !signing_session.is_writable() {
            return Err(MosaicError::SigningSessionAccountMustBeWritable.into());
        }
        if !signing_session.is_data_empty() {
            return Err(MosaicError::SigningSessionAccountMustNotBeInitialized.into());
        }

        Ok(Self {
            payer,
            root,
            signing_session,
        })
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct InitializeSigningSessionIxData {
    pub instruction_data: Vec<u8>,
    pub instruction_accounts: Vec<Vec<u8>>,
    pub bump: u8,
}

impl<'info> TryFrom<&'info [u8]> for InitializeSigningSessionIxData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        Ok(borsh::from_slice::<Self>(&data).map_err(|_| ProgramError::InvalidInstructionData)?)
    }
}

pub struct InitializeSigningSession<'info> {
    pub accounts: InitializeSigningSessionIxAccounts<'info>,
    pub instruction_data: InitializeSigningSessionIxData,
}

impl<'info> TryFrom<(&'info [AccountView], &'info [u8])> for InitializeSigningSession<'info> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'info [AccountView], &'info [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeSigningSessionIxAccounts::try_from(accounts)?;
        let instruction_data = InitializeSigningSessionIxData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> InitializeSigningSession<'info> {
    pub fn handler(&mut self) -> ProgramResult {
        let mut root_account = self.accounts.root.try_borrow_mut()?;
        let mut root_data = Root::unpack(&root_account)?;

        root_data.increment_last_id()?;

        signing_session_pda_check(
            &self.accounts.signing_session.address(),
            self.accounts.root.address().as_array(),
            root_data.last_id,
            &[self.instruction_data.bump],
        )?;

        root_pda_check(&self.accounts.root.address(), &[root_data.bump])?;
        Self::mandatory_account_data_checks(&root_data, self.accounts.payer.address())?;

        let derivation_new_last_session = &root_data.last_id.to_be_bytes();

        let signing_session_ix_data_bump = [self.instruction_data.bump];
        let signing_session_seeds = [
            Seed::from(self.accounts.root.address().as_ref()),
            Seed::from(derivation_new_last_session),
            Seed::from(SIGNING_SESSION_PDA),
            Seed::from(&signing_session_ix_data_bump),
        ];
        let cpi_signer = Signer::from(&signing_session_seeds);

        let (signing_session_data, signing_session_data_len) = SigningSession::init(
            self.instruction_data.clone(),
            root_data.last_id,
            self.accounts.root.address(),
        )
        .pack()?;

        // create signing session account
        pinocchio_system::instructions::CreateAccount {
            from: self.accounts.payer,
            to: self.accounts.signing_session,
            space: signing_session_data_len as u64,
            lamports: Rent::get()?.try_minimum_balance(signing_session_data_len)?,
            owner: &ID.into(),
        }
        .invoke_signed(&[cpi_signer])?;

        // write updated root state
        let (updated_root_data, updated_root_data_len) = root_data.pack()?;
        root_account[..updated_root_data_len].copy_from_slice(&updated_root_data);

        // write to signing session account
        let mut signing_data = self.accounts.signing_session.try_borrow_mut()?;
        signing_data[..signing_session_data.len()].copy_from_slice(&signing_session_data);

        Ok(())
    }

    #[must_use]
    fn mandatory_account_data_checks(root: &Root, signer: &Address) -> Result<(), ProgramError> {
        root.signer_must_be_operator(signer)?;

        Ok(())
    }
}
