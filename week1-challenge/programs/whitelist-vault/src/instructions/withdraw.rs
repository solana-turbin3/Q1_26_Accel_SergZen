use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::VaultError;
use crate::state::{UserDeposit, Vault};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [b"mint"],
        bump = vault.bump_mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
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
        seeds = [b"user_deposit".as_ref(), user.key().as_ref()],
        bump = user_deposit.bump,
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: extra account metas for transfer hook
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    /// CHECK: whitelist source owner
    #[account(
        seeds = [b"whitelist", vault.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    pub source_whitelist: UncheckedAccount<'info>,

    /// CHECK: whitelist destination owner  
    #[account(
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    pub destination_whitelist: UncheckedAccount<'info>,
    /// CHECK: transfer hook program
    pub hook_program: UncheckedAccount<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let user_balance = self.get_user_balance()?;

        require!(amount > 0, VaultError::InvalidAmount);
        require!(amount <= user_balance, VaultError::InsufficientFunds);

        self.sub_vault_amount(amount)?;

        self.sub_deposited_amount(amount)?;

        self.transfer_tokens(amount)
    }

    fn get_user_balance(&self) -> Result<u64> {
        let user_deposit = &self.user_deposit;

        Ok(user_deposit.amount)
    }

    fn sub_vault_amount(&mut self, amount: u64) -> Result<()> {
        let vault = &mut self.vault;
        vault.balance = vault
            .balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;

        Ok(())
    }

    fn sub_deposited_amount(&mut self, amount: u64) -> Result<()> {
        let user_deposit = &mut self.user_deposit;

        user_deposit.amount = user_deposit
            .amount
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;

        Ok(())
    }

    fn transfer_tokens(&mut self, amount: u64) -> Result<()> {
        let accounts = vec![
            AccountMeta::new(self.vault_ata.key(), false),
            AccountMeta::new_readonly(self.mint.key(), false),
            AccountMeta::new(self.user_ata.key(), false),
            AccountMeta::new(self.vault.key(), true),
            AccountMeta::new_readonly(self.extra_account_meta_list.key(), false),
            AccountMeta::new_readonly(self.source_whitelist.key(), false),
            AccountMeta::new_readonly(self.destination_whitelist.key(), false),
            AccountMeta::new_readonly(self.hook_program.key(), false),
        ];

        let mut data = vec![12];
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(self.mint.decimals);

        let ix = Instruction {
            program_id: self.token_program.key(),
            accounts,
            data,
        };

        let account_infos = vec![
            self.vault_ata.to_account_info(),
            self.mint.to_account_info(),
            self.user_ata.to_account_info(),
            self.vault.to_account_info(),
            self.extra_account_meta_list.to_account_info(),
            self.source_whitelist.to_account_info(),
            self.destination_whitelist.to_account_info(),
            self.hook_program.to_account_info(),
        ];

        let signer_seeds: &[&[&[u8]]] = &[&[b"vault", &[self.vault.bump]]];

        invoke_signed(&ix, &account_infos, signer_seeds)?;

        Ok(())
    }
}
