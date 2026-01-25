use crate::{
    ID, ROOT_PDA,
    errors::MosaicError,
    instructions::{root_pda_check, signing_session_pda_check},
    invoke_signed_dynamic,
    state::{
        PackUnpack,
        root::Root,
        signing_session::{InstructionAccount, SigningSession},
    },
};
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer, invoke_signed},
    error::ProgramError,
    instruction::{InstructionAccount as PinocchioInstructionAccount, InstructionView},
};

/// Execute Instruction
///
/// ### accounts:
///   0. `[WRITE, SIGNER]` payer
///   1. `[READ]`   root pda
///   2. `[WRITE]`  signing pda
///   3. `[READ]`   system program
///   4. `[READ]`   destination program
///   [..]          CPI accounts
pub struct ExecuteIxAccounts<'info> {
    pub payer: &'info AccountView,
    pub root: &'info AccountView,
    pub signing_session: &'info AccountView,
    pub _sys_program: &'info AccountView,
    pub _dst_program: &'info AccountView,
    pub remaining: &'info [AccountView],
}

impl<'info> TryFrom<&'info [AccountView]> for ExecuteIxAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        // perform accounts attribute check
        let (required_accounts, remaining) = accounts.split_at(5);
        let [payer, root, signing_session, _sys_program, _dst_program] = required_accounts else {
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
            _sys_program,
            _dst_program,
            remaining,
        })
    }
}

#[derive(Clone, Copy, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ExecuteIxData {}

impl<'info> TryFrom<&'info [u8]> for ExecuteIxData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        Ok(borsh::from_slice::<Self>(&data).map_err(|_| ProgramError::InvalidInstructionData)?)
    }
}

pub struct Execute<'info> {
    pub accounts: ExecuteIxAccounts<'info>,
    pub instruction_data: ExecuteIxData,
}

impl<'info> TryFrom<(&'info [AccountView], &'info [u8])> for Execute<'info> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'info [AccountView], &'info [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = ExecuteIxAccounts::try_from(accounts)?;
        let instruction_data = ExecuteIxData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> Execute<'info> {
    pub fn handler(&mut self) -> ProgramResult {
        let root_account = self.accounts.root.try_borrow()?;
        let root_data = Root::unpack(&root_account)?;
        let root_pda = Address::create_program_address(&[ROOT_PDA, &[root_data.bump]], &ID.into())
            .map_err(|_| ProgramError::InvalidSeeds)?;

        let root_bump_seed = [root_data.bump];
        let root_seed = [Seed::from(ROOT_PDA), Seed::from(&root_bump_seed)];
        let cpi_signer = Signer::from(&root_seed);

        let signing_data = {
            let signing_account = self.accounts.signing_session.try_borrow()?;
            SigningSession::unpack(&signing_account)?
        };

        root_pda_check(&self.accounts.root.address(), &[root_data.bump])?;
        signing_session_pda_check(
            &self.accounts.signing_session.address(),
            self.accounts.root.address().as_array(),
            root_data.last_id,
            &[signing_data.bump],
        )?;
        Self::mandatory_account_data_checks(
            &signing_data,
            &root_data,
            self.accounts._dst_program.address(),
        )?;

        // dynamic metas; allows for mapping accounts stored in signing session account
        let mut instruction_accounts: Vec<PinocchioInstructionAccount> = vec![];
        let mut addresses = vec![];

        for instruction_account_data in &signing_data.instruction_accounts {
            let ix_acc = InstructionAccount::deserialize(instruction_account_data)?;
            addresses.push(Address::new_from_array(ix_acc.pubkey));
        }

        for (i, instruction_account_data) in signing_data.instruction_accounts.iter().enumerate() {
            let ix_acc = InstructionAccount::deserialize(instruction_account_data)?;
            let pinocchio_acc = match (ix_acc.signer, ix_acc.writable) {
                (true, true) => PinocchioInstructionAccount::writable_signer(&addresses[i]),
                (true, false) => PinocchioInstructionAccount::readonly_signer(&addresses[i]),
                (false, true) => PinocchioInstructionAccount::writable(&addresses[i]),
                (false, false) => PinocchioInstructionAccount::readonly(&addresses[i]),
            };
            instruction_accounts.push(pinocchio_acc);
        }

        let mut account_views: Vec<&AccountView> = vec![];
        for address in addresses.iter() {
            if address.as_ref() == root_pda.as_ref() {
                account_views.push(self.accounts.root);
            } else {
                let found = self
                    .accounts
                    .remaining
                    .iter()
                    .find(|acc| acc.address().as_ref() == address.as_ref())
                    .ok_or(ProgramError::NotEnoughAccountKeys)?;
                account_views.push(found);
            }
        }

        // cpi to destination program
        let instruction = InstructionView {
            program_id: self.accounts._dst_program.address(),
            accounts: &instruction_accounts,
            data: &signing_data.instruction_data.clone(),
        };
        invoke_signed_dynamic!(&instruction, account_views, &[cpi_signer])?;

        // update signing session / prevent re-execution
        let mut signing_data = signing_data;
        signing_data.progress_phase_checked()?; /* set signing session phase to executed */

        let (serialized_data, serialized_len) = signing_data.pack()?;
        let mut signing_account = self.accounts.signing_session.try_borrow_mut()?;
        signing_account[..serialized_len].copy_from_slice(&serialized_data);

        Ok(())
    }

    #[must_use]
    fn mandatory_account_data_checks(
        signing_session: &SigningSession,
        root: &Root,
        ix_provided_destination_program: &Address,
    ) -> Result<(), ProgramError> {
        signing_session.sessions_must_equal(root.last_id)?;
        signing_session.must_be_approved()?;
        root.destination_program_address_must_match(ix_provided_destination_program)?;
        Ok(())
    }
}
