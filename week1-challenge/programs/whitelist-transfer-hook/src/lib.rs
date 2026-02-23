#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use crate::instructions::*;

use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::{
    instruction::{
        ExecuteInstruction, 
    },
};

pub mod errors;
pub mod instructions;
pub mod state;

declare_id!("CHVVjSRy3TBZErMPk2hpwbyMxNJivHoFpLsHCvAHVtgg");

pub use ID as WHITELIST_TRANSFER_HOOK_ID;

#[program]
pub mod whitelist_transfer_hook {
    use super::*;
    
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize_transfer_hook()?;
        ctx.accounts.initialize_config(&ctx.bumps)
    }

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.add_to_whitelist(user, &ctx.bumps)
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist(user)
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}