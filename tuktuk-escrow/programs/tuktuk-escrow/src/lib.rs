#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;

mod state;
mod instructions;
mod errors;
mod constants;

use instructions::*;

declare_id!("CSt7PWeXZDohtCHubQQMug1HoZf2L8dx5zhtWpyQ8zQr");

#[program]
pub mod tuktuk_escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        ctx.accounts.init_escrow(seed, receive, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    pub fn refund_auto(ctx: Context<RefundAuto>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit()?;
        ctx.accounts.withdraw_and_close_vault()
    }

    pub fn schedule(ctx: Context<Schedule>, task_id: u16) -> Result<()> {
        ctx.accounts.schedule(task_id, &ctx.bumps)
    }
}