use pinocchio::{AccountView, error::ProgramError};
use wincode::SchemaRead;

#[derive(SchemaRead)]
pub struct Contributor {
    pub amount: [u8; 8],
    pub bump: u8,
}

impl Contributor {
    pub const LEN: usize = 8 + 1;

    pub fn from_account_info(account_info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() != Contributor::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn add_amount(&mut self, amount: u64) -> Result<(), ProgramError> {
        let current = u64::from_le_bytes(self.amount);
        self.amount = current
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }
}