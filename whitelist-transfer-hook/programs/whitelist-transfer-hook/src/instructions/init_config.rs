use anchor_lang::prelude::*;

use crate::{errors::WhitelistTransferHookError, state::Config};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = bpf_loader_upgradeable::id(),
        constraint = program_data.upgrade_authority_address.is_some_and(|auth| auth == admin.key()) @ WhitelistTransferHookError::Unauthorized,
    )]
    pub program_data: Account<'info, ProgramData>,
}

impl<'info> InitializeConfig<'info> {
    pub fn initialize_config(&mut self, bumps: &InitializeConfigBumps) -> Result<()> {
        self.config.set_inner(Config {
            admin: self.admin.key(),
            bump: bumps.config,
        });

        Ok(())
    }
}