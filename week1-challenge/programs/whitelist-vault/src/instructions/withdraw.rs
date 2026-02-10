use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::{transfer_checked, TransferChecked, Token2022};
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::VaultError;
use crate::state::{Vault, Whitelist};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault".as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        seeds = [b"whitelist".as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}


impl<'info> Withdraw<'info> {

    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let user_balance = self.get_user_balance()?;

        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            amount <= user_balance,
            VaultError::InsufficientFunds
        );

        let vault = &mut self.vault;
        vault.balance = vault
            .balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;

        self.update_whitelisted_amounts(amount)?;

        let cpi_accounts = TransferChecked {
            from: self.vault_ata.to_account_info(),
            to: self.user_ata.to_account_info(),
            authority: self.user.to_account_info(),
            mint: self.mint.to_account_info(),
        };
        
        let signer_seeds: [&[&[u8]]; 1] = [&[b"vault".as_ref(), &[self.vault.bump]]];

        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer_seeds);
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        Ok(())
    }

    fn get_user_balance(&self) -> Result<u64> {
        let whitelist = &self.whitelist;
        let index = whitelist
            .get_index(self.user.key())
            .ok_or(VaultError::InvalidWhitelistAccount)?;
        Ok(whitelist.amount[index])
    }
    fn update_whitelisted_amounts(&mut self, amount: u64) -> Result<()> {
        let whitelist = &mut self.whitelist;
        let index = whitelist
            .get_index(self.user.key())
            .ok_or(VaultError::InvalidWhitelistAccount)?;

        whitelist.amount[index] = whitelist.amount[index]
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;

        Ok(())
    }
}
