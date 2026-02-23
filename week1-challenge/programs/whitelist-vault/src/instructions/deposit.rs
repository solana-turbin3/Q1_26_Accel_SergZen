use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::{Token2022};
use anchor_spl::token_interface::{Mint, TokenAccount};
use anchor_lang::solana_program::instruction::{Instruction, AccountMeta};
use spl_transfer_hook_interface::solana_cpi::invoke;

use crate::errors::VaultError;
use crate::state::{Vault, UserDeposit};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        init_if_needed,
        payer = user,
        seeds = [b"user_deposit".as_ref(), user.key().as_ref()],
        space = UserDeposit::DISCRIMINATOR.len() + UserDeposit::INIT_SPACE,
        bump,
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
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    pub source_whitelist: UncheckedAccount<'info>,

    /// CHECK: whitelist destination owner  
    #[account(
        seeds = [b"whitelist", vault.key().as_ref()],
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

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, bumps: DepositBumps, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        self.add_vault_amount(amount)?;

        self.add_deposited_amount(bumps, amount)?;

        self.transfer_tokens(amount)
    }

    fn add_vault_amount(&mut self, amount: u64) -> Result<()> {
        let vault = &mut self.vault;
        vault.balance = vault
            .balance
            .checked_add(amount)
            .ok_or(VaultError::OverflowError)?;

        Ok(())
    }

    fn add_deposited_amount(&mut self, bumps: DepositBumps, amount: u64) -> Result<()> {
        let user_deposit = &mut self.user_deposit;

        if user_deposit.address == Pubkey::default() {
            user_deposit.address = self.user.key();
            user_deposit.amount = 0;
            user_deposit.bump = bumps.user_deposit;
        }

        user_deposit.amount = user_deposit
            .amount
            .checked_add(amount)
            .ok_or(VaultError::OverflowError)?;

        Ok(())
    }

    fn transfer_tokens(&mut self, amount: u64) -> Result<()> {
        let accounts = vec![
            AccountMeta::new(self.user_ata.key(), false),
            AccountMeta::new_readonly(self.mint.key(), false),
            AccountMeta::new(self.vault_ata.key(), false),
            AccountMeta::new(self.user.key(), true),
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
            self.user_ata.to_account_info(),
            self.mint.to_account_info(),
            self.vault_ata.to_account_info(),
            self.user.to_account_info(),
            self.extra_account_meta_list.to_account_info(),
            self.source_whitelist.to_account_info(),
            self.destination_whitelist.to_account_info(),
            self.hook_program.to_account_info(),
        ];

        invoke(&ix, &account_infos)?;

        Ok(())
    }
}
