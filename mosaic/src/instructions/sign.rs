use crate::{
    ID,
    errors::MosaicError,
    instructions::{root_pda_check, signing_session_pda_check},
    state::{PackUnpack, root::Root, signing_session::SigningSession},
};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    AccountView, Address, ProgramResult,
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};

/// Sign Session
///
/// ### accounts:
///   0. `[WRITE, SIGNER]`
///   1. `[WRITE]`  root pda
///   2. `[WRITE]`  signing session pda
pub struct SignIxAccounts<'info> {
    pub payer: &'info AccountView,
    pub root: &'info AccountView,
    pub signing_session: &'info AccountView,
}

impl<'info> TryFrom<&'info [AccountView]> for SignIxAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        // perform accounts attribute check
        let [payer, root, signing_session, _system_program] = accounts else {
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

        Ok(Self {
            payer,
            root,
            signing_session,
        })
    }
}

#[derive(Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct SignIxData {
    pub bump: u8,
}

impl<'info> TryFrom<&'info [u8]> for SignIxData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        Ok(borsh::from_slice::<Self>(&data).map_err(|_| ProgramError::InvalidInstructionData)?)
    }
}

pub struct Sign<'info> {
    pub accounts: SignIxAccounts<'info>,
    pub instruction_data: SignIxData,
}

impl<'info> TryFrom<(&'info [AccountView], &'info [u8])> for Sign<'info> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'info [AccountView], &'info [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = SignIxAccounts::try_from(accounts)?;
        let instruction_data = SignIxData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> Sign<'info> {
    pub fn handler(&mut self) -> ProgramResult {
        let root_account = &self.accounts.root.try_borrow()?;
        let root_data = Root::unpack(&root_account)?;

        root_pda_check(&self.accounts.root.address(), &[root_data.bump])?;
        signing_session_pda_check(
            &self.accounts.signing_session.address(),
            self.accounts.root.address().as_array(),
            root_data.last_id,
            &[self.instruction_data.bump],
        )?;

        let signing_account = self.accounts.signing_session.try_borrow()?;
        let mut signing: SigningSession = SigningSession::unpack(&signing_account)?;
        let signing_current_account_size = signing_account.len();
        drop(signing_account);

        Self::mandatory_checks(&signing, &root_data, self.accounts.payer.address())?;
        signing.approve_checked(self.accounts.payer.address())?;

        if signing.check_approvals_reaching_threshold(root_data.threshold.into()) {
            signing.progress_phase_checked()?;
        }
        let (signing, new_signing_len) = signing.pack()?;

        if new_signing_len != signing_current_account_size {
            let rent = Rent::get()?;
            let new_minimum_balance = rent.try_minimum_balance(new_signing_len)?;
            let current_lamports = self.accounts.signing_session.lamports();

            // top-up missing rent
            if current_lamports < new_minimum_balance {
                pinocchio_system::instructions::Transfer {
                    from: self.accounts.payer,
                    to: self.accounts.signing_session,
                    lamports: new_minimum_balance - current_lamports,
                }
                .invoke()?;
            }
            self.accounts.signing_session.resize(new_signing_len)?;

            let mut signing_account = self.accounts.signing_session.try_borrow_mut()?;
            signing_account[..new_signing_len].copy_from_slice(&signing);
        }

        Ok(())
    }

    #[must_use]
    fn mandatory_checks(
        signing: &SigningSession,
        root: &Root,
        signer: &Address,
    ) -> Result<(), ProgramError> {
        signing.must_be_active()?;
        root.signer_must_be_operator(signer)?;

        Ok(())
    }
}
