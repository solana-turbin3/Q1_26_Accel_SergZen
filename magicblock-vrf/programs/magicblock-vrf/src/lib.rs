#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod state;
mod instructions;

use instructions::*;

declare_id!("9rLmyEBSfFgAKfSQdgkbsXmh34cVwdirVTbRDME31z6Z");

#[ephemeral]
#[program]
pub mod magicblock_vrf {

    use super::*;

    pub fn initialize(ctx: Context<InitUser>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;
        
        Ok(())
    }

    pub fn update(ctx: Context<UpdateUser>, new_data: u64) -> Result<()> {
        ctx.accounts.update(new_data)?;
        
        Ok(())
    }

    pub fn update_commit(ctx: Context<UpdateCommit>, new_data: u64) -> Result<()> {
        ctx.accounts.update_commit(new_data)?;
        
        Ok(())
    }

    pub fn delegate(ctx: Context<Delegate>) -> Result<()> {
        ctx.accounts.delegate()?;
        
        Ok(())
    }

    pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
        ctx.accounts.undelegate()?;
        
        Ok(())
    }

    pub fn close(ctx: Context<CloseUser>) -> Result<()> {
        ctx.accounts.close()?;
        
        Ok(())
    }

    pub fn request_randomness(ctx: Context<RequestRandomness>, client_seed: u8) -> Result<()> {
        ctx.accounts.request_randomness(client_seed)?;

        Ok(())
    }

    pub fn callback_randomness(
        ctx: Context<CallbackRandomness>,
        randomness: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.callback_randomness(randomness)?;

        Ok(())
    }
}

