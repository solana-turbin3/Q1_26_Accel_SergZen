use anchor_lang::prelude::*;
use mpl_core::{ID as MPL_CORE_ID, accounts::{BaseAssetV1, BaseCollectionV1}, instructions::TransferV1CpiBuilder, types::UpdateAuthority};

use crate::{errors::StakingError, state::Oracle};

#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: This is safe
    pub new_owner: UncheckedAccount<'info>,
    /// CHECK: NFT account will be checked by the mpl core program
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account will be checked by the mpl core program
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"oracle", collection.key().as_ref()],
        bump  = oracle.bump,
        has_one = collection,
    )]
    pub oracle: Account<'info, Oracle>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is the ID of the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> TransferNft<'info> {
    pub fn transfer_nft(&mut self) -> Result<()> {
        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.owner.key(), StakingError::InvalidOwner);
        require!(base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()), StakingError::InvalidAuthority);
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .new_owner(&self.new_owner.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .authority(Some(&self.owner.to_account_info()))
            .add_remaining_account(&self.oracle.to_account_info(), false, false)
            .invoke()?;

        Ok(())
    }
}