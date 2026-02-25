use anchor_lang::prelude::*;

use solana_gpt_oracle::Identity;

#[derive(Accounts)]
pub struct CallbackAgent<'info> {
    /// CHECK: Checked in oracle program
    pub identity: Account<'info, Identity>,
}

impl<'info> CallbackAgent<'info> {
    pub fn callback_from_agent(&mut self, response: String) -> Result<()> {
        if !self.identity.to_account_info().is_signer {
            return Err(ProgramError::InvalidAccountData.into());
        }

        msg!("GPT Agent Response: {:?}", response);

        Ok(())
    }
}