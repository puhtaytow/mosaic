use crate::{
    ID, SIGNING_SESSION_PDA,
    errors::MosaicError,
    instructions::{initialize_pda_account, root_pda_check, signing_session_pda_check},
    state::{PackUnpack, root::Root, signing_session::SigningSession},
};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
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
        if !payer.is_writable() {
            return Err(MosaicError::PayerAccountMustBeWritable.into());
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
        if !signing_session.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::IllegalOwner);
        }
        if payer.address() == signing_session.address() {
            return Err(ProgramError::InvalidArgument);
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
        let root_account = self.accounts.root.try_borrow()?;
        root_pda_check(
            &self.accounts.root.address(),
            &[*root_account.last().unwrap()],
        )?;

        let mut root_data = Root::unpack(&root_account)?;
        Self::mandatory_checks(&root_data, self.accounts.payer.address())?;
        drop(root_account);

        root_data.increment_last_id()?;

        signing_session_pda_check(
            &self.accounts.signing_session.address(),
            self.accounts.root.address().as_array(),
            root_data.last_id,
            &[self.instruction_data.bump],
        )?;

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
        let (root_data, root_data_len) = root_data.pack()?;

        initialize_pda_account(
            self.accounts.payer,
            self.accounts.signing_session,
            signing_session_data_len,
            &[cpi_signer],
        )?;

        let mut root_account = self.accounts.root.try_borrow_mut()?;
        let mut signing_data = self.accounts.signing_session.try_borrow_mut()?;

        root_account[..root_data_len].copy_from_slice(&root_data);
        signing_data[..signing_session_data.len()].copy_from_slice(&signing_session_data);

        Ok(())
    }

    #[must_use]
    fn mandatory_checks(root: &Root, signer: &Address) -> Result<(), ProgramError> {
        root.signer_must_be_operator(signer)?;
        Ok(())
    }
}
