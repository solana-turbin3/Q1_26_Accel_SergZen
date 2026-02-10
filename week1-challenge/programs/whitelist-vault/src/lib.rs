#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use crate::instructions::*;
use spl_discriminator::discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

pub mod errors;
pub mod instructions;
pub mod state;
pub mod tests;


declare_id!("2Ze9h7UzmTccSf5F6oYYrxxM6biDMDPUWh2B1iKwubEg");


#[program]
pub mod whitelist_transfer_hook {
    use super::*;
    
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(ctx.bumps)
    }


    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts
            .deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }


    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}
