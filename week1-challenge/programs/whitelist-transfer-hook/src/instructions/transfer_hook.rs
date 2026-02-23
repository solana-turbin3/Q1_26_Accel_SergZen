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

use crate::errors::WhitelistTransferHookError;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token_ata: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub destination_token_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    /// CHECK: whitelist source owner
    #[account(
        seeds = [b"whitelist", source_token_ata.owner.as_ref()],
        bump,
    )]
    pub source_whitelist: UncheckedAccount<'info>,

    /// CHECK: whitelist для destination owner
    #[account(
        seeds = [b"whitelist", destination_token_ata.owner.as_ref()],
        bump,
    )]
    pub destination_whitelist: UncheckedAccount<'info>,
}

impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;

        let source_is_whitelisted = self.source_whitelist.data_len() > 0;
        let destination_is_whitelisted = self.destination_whitelist.data_len() > 0;

        if !source_is_whitelisted {
            require!(
                destination_is_whitelisted,
                WhitelistTransferHookError::NotWhitelisted
            );
        }

        Ok(())
    }

    fn check_is_transferring(&mut self) -> Result<()> {
        let source_token_info = self.source_token_ata.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        require!(
            bool::from(account_extension.transferring),
            WhitelistTransferHookError::NotTransferring
        );

        Ok(())
    }
}