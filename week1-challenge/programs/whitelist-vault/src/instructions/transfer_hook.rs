use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::errors::VaultError;
use crate::state::Whitelist;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token_ata: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub destination_token_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [b"whitelist".as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
}

impl<'info> TransferHook<'info> {
    /// This function is called when the transfer hook is executed.
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook

        self.check_is_transferring()?;

        let owner_key = self.owner.key();
        if self.whitelist.get_index(owner_key).is_some() {
            msg!("Transfer allowed: The address is whitelisted");
            return Ok(());
        }

        err!(VaultError::NotWhitelisted)
    }

    /// Checks if the transfer hook is being executed during a transfer operation.
    fn check_is_transferring(&mut self) -> Result<()> {
        // Ensure that the source token account has the transfer hook extension enabled

        // Get the account info of the source token account
        let source_token_info = self.source_token_ata.to_account_info();
        // Borrow the account data mutably
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;

        // Unpack the account data as a PodStateWithExtensionsMut
        // This will allow us to access the extensions of the token account
        // We use PodStateWithExtensionsMut because TokenAccount is a POD (Plain Old Data) type
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        // Get the TransferHookAccount extension
        // Search for the TransferHookAccount extension in the token account
        // The returning struct has a `transferring` field that indicates if the account is in the middle of a transfer operation
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        // Check if the account is in the middle of a transfer operation
        if !bool::from(account_extension.transferring) {
            panic!("TransferHook: Not transferring");
        }

        Ok(())
    }
}
