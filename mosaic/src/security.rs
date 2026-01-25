//! multi-layer security checks

use pinocchio::{AccountView, Address, error::ProgramError};
use std::marker::PhantomData;

pub struct Unvalidated;
pub struct AccountsValidated;
pub struct DataValidated;
pub struct Ready;

pub struct ValidationContext<'info> {
    pub program_id: &'info Address,
    pub accounts: &'info [AccountView],
    pub instruction_data: &'info [u8],
}

pub struct ValidationBuilder<'info, State> {
    context: ValidationContext<'info>,
    _state: PhantomData<State>,
}

impl<'info> ValidationBuilder<'info, Unvalidated> {
    pub fn new(
        program_id: &'info Address,
        accounts: &'info [AccountView],
        instruction_data: &'info [u8],
    ) -> Self {
        Self {
            context: ValidationContext {
                program_id,
                accounts,
                instruction_data,
            },
            _state: PhantomData,
        }
    }

    pub fn validate_accounts(
        self,
    ) -> Result<ValidationBuilder<'info, AccountsValidated>, ProgramError> {
        // TODO: perform accounts check
        Ok(ValidationBuilder {
            context: self.context,
            _state: PhantomData,
        })
    }
}

impl<'info> ValidationBuilder<'info, AccountsValidated> {
    pub fn validate_data(self) -> ValidationBuilder<'info, DataValidated> {
        // TODO: perform instruction data check
        Ok(ValidationBuilder {
            context: self.context,
            _state: PhantomData,
        })
    }
}

impl<'info> ValidationBuilder<'info, DataValidated> {
    pub fn ready(self) -> ValidationBuilder<'info, Ready> {
        Ok(ValidationBuilder {
            context: self.context,
            _state: PhantomData,
        })
    }
}
