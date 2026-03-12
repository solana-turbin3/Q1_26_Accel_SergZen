use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use mpl_core::{
    ID as MPL_CORE_ID, 
    accounts::BaseCollectionV1, 
    instructions::{AddCollectionExternalPluginAdapterV1CpiBuilder}, 
    types::{
        ExternalCheckResult, ExternalPluginAdapterInitInfo, 
        HookableLifecycleEvent, OracleInitInfo, ValidationResultsOffset
    }
};

use crate::{constants::VAULT_INITIAL_FUND, errors::StakingError, state::Oracle};

#[derive(Accounts)]
pub struct InitOracle<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
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
        init,
        payer = admin,
        space = Oracle::DISCRIMINATOR.len() + Oracle::INIT_SPACE,
        seeds = [b"oracle", collection.key().as_ref()],
        bump
    )]
    pub oracle: Account<'info, Oracle>,
    /// CHECK: lamport vault PDA
    #[account(
        mut,
        seeds = [b"reward_vault", oracle.key().as_ref()],
        bump
    )]
    pub reward_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is the ID of the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl InitOracle<'_> {
    pub fn init_oracle(&mut self, bumps: &InitOracleBumps) -> Result<()> {
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        let clock = Clock::get()?;
        let transfer_validation = Oracle::transfer_validation(clock.unix_timestamp);
        let validation = Oracle::make_validation(transfer_validation);

        self.oracle.set_inner(Oracle {
            validation,
            collection: self.collection.key(),
            last_updated: clock.unix_timestamp,
            bump: bumps.oracle,
            vault_bump: bumps.reward_vault
        });

        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                     from: self.admin.to_account_info(),
                     to:   self.reward_vault.to_account_info(),
                }
            ),
            VAULT_INITIAL_FUND,
        )?;

        let collection_key = self.collection.key();
        let signer_seeds: &[&[u8]] = &[
            b"update_authority",
            collection_key.as_ref(),
            &[bumps.update_authority],
        ];

        AddCollectionExternalPluginAdapterV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.admin.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .init_info(ExternalPluginAdapterInitInfo::Oracle(OracleInitInfo {
                base_address: self.oracle.key(),
                results_offset: Some(ValidationResultsOffset::Anchor),
                lifecycle_checks: vec![(
                    HookableLifecycleEvent::Transfer,
                    ExternalCheckResult { flags: 4 },
                )],
                init_plugin_authority: None,
                base_address_config: None,
            }))
            .invoke_signed(&[signer_seeds])?;

        Ok(())
    }
}