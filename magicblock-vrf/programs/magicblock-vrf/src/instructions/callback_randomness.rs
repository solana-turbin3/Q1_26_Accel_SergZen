use anchor_lang::prelude::*;

use ephemeral_vrf_sdk::rnd::random_u64;

use crate::state::UserAccount;

#[derive(Accounts)]
pub struct CallbackRandomness<'info> {
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,

    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> CallbackRandomness<'info> {
    pub fn callback_randomness(&mut self, randomness: [u8; 32]) -> Result<()> {
        msg!("Callback randomness...");

        let rnd = random_u64(&randomness);

        self.user_account.random = rnd;

        Ok(())
    }
}