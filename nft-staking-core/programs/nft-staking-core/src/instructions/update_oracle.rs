use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use mpl_core::accounts::BaseCollectionV1;

use crate::{
    constants::CRANK_REWARD_LAMPORTS, errors::StakingError, state::{Oracle}
};

#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
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
    #[account(
        mut,
        seeds = [b"reward_vault", oracle.key().as_ref()],
        bump = oracle.vault_bump,
    )]
    pub reward_vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> UpdateOracle<'info> {
    pub fn update_oracle(&mut self) -> Result<()> {
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        let timestamp = Clock::get()?.unix_timestamp;
        let new_transfer_validation = Oracle::transfer_validation(timestamp);
        require!(
            self.oracle.validation != Oracle::make_validation(new_transfer_validation),
            StakingError::AlreadyUpdated
        );

        self.oracle.validation = Oracle::make_validation(new_transfer_validation);

        let reward_vault_lamports = self.reward_vault.lamports();
        let oracle_key = self.oracle.key();
        let signer_seeds = &[b"reward_vault", oracle_key.as_ref(), &[self.oracle.vault_bump]];

        if Oracle::is_boundary_time(timestamp) && reward_vault_lamports > CRANK_REWARD_LAMPORTS
        {
            transfer(
                CpiContext::new_with_signer(
                    self.system_program.to_account_info(),
                    Transfer {
                        from: self.reward_vault.to_account_info(),
                        to: self.cranker.to_account_info(),
                    },
                    &[signer_seeds],
                ),
                CRANK_REWARD_LAMPORTS,
            )?
        }

        Ok(())
    }

}