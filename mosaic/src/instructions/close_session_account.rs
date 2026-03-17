use crate::{
    ID,
    errors::MosaicError,
    instructions::{root_pda_check, signing_session_pda_check},
    state::{PackUnpack, root::Root, signing_session::SigningSession},
};
use pinocchio::{AccountView, ProgramResult, error::ProgramError};

/// Close Session Account
///
/// ### accounts:
///   0. `[WRITE, SIGNER]` payer (lamports recipient)
///   1. `[READ]` root pda
///   2. `[WRITE]` signing session pda
pub struct CloseSessionAccountIxAccounts<'info> {
    pub payer: &'info AccountView,
    pub root: &'info AccountView,
    pub signing_session: &'info AccountView,
}

impl<'info> TryFrom<&'info [AccountView]> for CloseSessionAccountIxAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        let [payer, root, signing_session] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !root.owned_by(&ID.into()) {
            return Err(MosaicError::RootAccountIncorrectOwner.into());
        }
        if !signing_session.owned_by(&ID.into()) {
            return Err(MosaicError::SigningSessionAccountIncorrectOwner.into());
        }
        if !payer.is_signer() {
            return Err(MosaicError::PayerMustEqualSigner.into());
        }
        if !signing_session.is_writable() {
            return Err(MosaicError::SigningSessionAccountMustBeWritable.into());
        }
        if signing_session.is_data_empty() {
            return Err(MosaicError::SigningSessionAccountMustBeInitialized.into());
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

#[derive(Clone, Copy, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct CloseSessionAccountIxData {}

impl<'info> TryFrom<&'info [u8]> for CloseSessionAccountIxData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        Ok(borsh::from_slice::<Self>(&data).map_err(|_| ProgramError::InvalidInstructionData)?)
    }
}

pub struct CloseSessionAccount<'info> {
    pub accounts: CloseSessionAccountIxAccounts<'info>,
    pub instruction_data: CloseSessionAccountIxData,
}

impl<'info> TryFrom<(&'info [AccountView], &'info [u8])> for CloseSessionAccount<'info> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'info [AccountView], &'info [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = CloseSessionAccountIxAccounts::try_from(accounts)?;
        let instruction_data = CloseSessionAccountIxData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> CloseSessionAccount<'info> {
    pub fn handler(&mut self) -> ProgramResult {
        let root_data = {
            let root_account = self.accounts.root.try_borrow()?;
            Root::unpack(&root_account)?
        };
        // TODO: this could be optimized to read just single byte to avoid deserializtion cost
        let signing_data = {
            let signing_account = self.accounts.signing_session.try_borrow()?;
            SigningSession::unpack(&signing_account)?
        };

        root_pda_check(&self.accounts.root.address(), &[root_data.bump])?;
        signing_session_pda_check(
            &self.accounts.signing_session.address(),
            self.accounts.root.address().as_array(),
            signing_data.session_id,
            &[signing_data.bump],
        )?;
        Self::mandatory_checks(&root_data, &signing_data, self.accounts.payer.address())?;

        let signing_lamports = self.accounts.signing_session.lamports();
        let payer_new_lamports = self
            .accounts
            .payer
            .lamports()
            .checked_add(signing_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.accounts.payer.set_lamports(payer_new_lamports);
        self.accounts.signing_session.close()?;

        Ok(())
    }

    #[must_use]
    fn mandatory_checks(
        root: &Root,
        signing_session: &SigningSession,
        signer: &pinocchio::Address,
    ) -> Result<(), ProgramError> {
        root.signer_must_be_operator(signer)?;
        signing_session.must_be_executed()?;
        Ok(())
    }
}
