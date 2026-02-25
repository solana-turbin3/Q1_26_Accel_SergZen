use anchor_lang::prelude::*;

declare_id!("7RzQCMYVgvovHjbvMQW1PZQM3fHvq3bn6BamMSrRswDB");

mod state;
mod instructions;
mod constants;
pub use instructions::*;

#[program]
pub mod tuktuk_gpt_oracle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn interact_agent(ctx: Context<InteractAgent>, text: String) -> Result<()> {
        ctx.accounts.interact_agent(text)
    }

    pub fn callback_agent(ctx: Context<CallbackAgent>, response: String) -> Result<()> {
        ctx.accounts.callback_from_agent(response)
    }

    pub fn schedule(ctx: Context<Schedule>, text: String, task_id: u16) -> Result<()> {
        ctx.accounts.schedule(text, task_id, &ctx.bumps)
    }
}
