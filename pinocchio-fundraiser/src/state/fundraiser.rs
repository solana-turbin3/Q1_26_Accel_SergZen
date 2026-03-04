use pinocchio::{AccountView, error::ProgramError};
use wincode::SchemaRead;

use crate::constants::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, SchemaRead)]
pub struct Fundraiser {
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
}

impl Fundraiser {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1;

    pub fn from_account_info(account_info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() != Fundraiser::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn amount_to_raise(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_raise)
    }

    pub fn current_amount(&self) -> u64 {
        u64::from_le_bytes(self.current_amount)
    }

    pub fn time_started(&self) -> i64 {
        i64::from_le_bytes(self.time_started)
    }

    pub fn max_contribution(&self) -> u64 {
        self.amount_to_raise() * MAX_CONTRIBUTION_PERCENTAGE / PERCENTAGE_SCALER
    }

    pub fn add_current_amount(&mut self, amount: u64) -> Result<(), ProgramError> {
        let current = u64::from_le_bytes(self.current_amount);
        self.current_amount = current
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }

    pub fn sub_current_amount(&mut self, amount: u64) -> Result<(), ProgramError> {
        let current = u64::from_le_bytes(self.current_amount);
        self.current_amount = current
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }
}
