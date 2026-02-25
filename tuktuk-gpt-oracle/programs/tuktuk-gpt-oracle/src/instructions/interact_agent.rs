use anchor_lang::prelude::*;

use solana_gpt_oracle::{ContextAccount};

use crate::state::Agent;

#[derive(Accounts)]
pub struct InteractAgent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Checked in oracle program
    #[account(mut)]
    pub interaction: UncheckedAccount<'info>,

    #[account(
        seeds = [b"agent", payer.key().as_ref()], 
        bump = agent.bump,
    )]
    pub agent: Account<'info, Agent>,

    #[account(address = agent.context)]
    pub context_account: Account<'info, ContextAccount>,

    /// CHECK: Checked oracle id
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> InteractAgent<'info> {
    pub fn interact_agent(&mut self, text: String) -> Result<()> {
        let cpi_program = self.oracle_program.to_account_info();

        let cpi_accounts = solana_gpt_oracle::cpi::accounts::InteractWithLlm {
            payer: self.payer.to_account_info(),
            interaction: self.interaction.to_account_info(),
            context_account: self.context_account.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        let callback_disc: [u8; 8] = crate::instruction::CallbackAgent::DISCRIMINATOR
            .try_into()
            .expect("Discriminator must be 8 bytes");

        solana_gpt_oracle::cpi::interact_with_llm(
            cpi_ctx, 
            text, 
            crate::ID, 
            callback_disc, 
            None
        )?;

        Ok(())
    }
}