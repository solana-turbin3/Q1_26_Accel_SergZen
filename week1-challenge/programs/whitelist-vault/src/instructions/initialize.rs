use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::{Token2022};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::state::{Vault};

#[derive(Accounts)]
pub struct Initialize <'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [b"mint"],
        bump,
        mint::decimals = 9,
        mint::authority = mint,
        mint::token_program = token_program,

        extensions::transfer_hook::authority = admin,
        extensions::transfer_hook::program_id = whitelist_transfer_hook::ID,

        extensions::close_authority::authority = admin
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        seeds = [b"vault".as_ref()],
        bump,
        payer = admin,
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        associated_token::mint = mint,
        associated_token::authority = vault,
        payer = admin,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: InitializeBumps) -> Result<()> {
        self.initialize_vault(bumps)?;

        Ok(())
    }

    fn initialize_vault(&mut self, bumps: InitializeBumps) -> Result<()> {
         self.vault.set_inner(Vault {
            admin: self.admin.key(),
            mint: self.mint.key(),
            bump_mint: bumps.mint,
            balance: 0,
            bump: bumps.vault,
        });

        Ok(())
    }
}
