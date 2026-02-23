use anchor_lang::prelude::*;

use crate::{errors::WhitelistTransferHookError, state::{Config, whitelist::Whitelist}};

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(
        mut,
        address = config.admin @ WhitelistTransferHookError::InvalidAdmin,
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config".as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = admin,
        space = Whitelist::DISCRIMINATOR.len() + Whitelist::INIT_SPACE,
        seeds = [b"whitelist", user.as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, user: Pubkey, bumps: &AddToWhitelistBumps) -> Result<()> {
        self.whitelist.set_inner(Whitelist {
            address: user,
            bump: bumps.whitelist,
        });

        msg!("Adding user to whitelist: {}", user);

        Ok(())
    }
}