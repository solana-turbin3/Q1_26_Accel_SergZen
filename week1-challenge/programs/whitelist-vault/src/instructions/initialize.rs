use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::instructions::InitializeExtraAccountMetaList;
use crate::state::{Vault, Whitelist};

#[derive(Accounts)]
pub struct Initialize <'info> {
    
    #[account(mut)]
    pub admin: Signer<'info>,
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
        seeds = [b"whitelist".as_ref()],
        bump,
        payer = admin,
        space = Whitelist::DISCRIMINATOR.len() + Whitelist::INIT_SPACE,
    )]
    pub whitelist: Account<'info, Whitelist>,

    #[account(
        init,
        payer = admin,
        mint::decimals = 9,
        mint::authority = admin,
        mint::token_program = token_program,
        extensions::transfer_hook::authority = admin,
        extensions::transfer_hook::program_id = crate::ID,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

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

    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        ).unwrap(),
        payer = admin
    )]
    /// CHECK: This account is initialized and populated via ExtraAccountMetaList::init.
    pub extra_account_meta_list: AccountInfo<'info>,


}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: InitializeBumps) -> Result<()> {
        self.initialize_vault(bumps.vault)?;
        self.initialize_whitelist(bumps.whitelist)?;
        self.initialize_transfer_hook()?;
        Ok(())
    }
    

    fn initialize_vault(&mut self, bump: u8) -> Result<()> {
        let vault = &mut self.vault;
        vault.admin = self.admin.key();
        vault.mint = self.mint.key();
        vault.bump = bump;
        vault.balance = 0;

        Ok(())
    }

    fn initialize_whitelist(&mut self, bump: u8) -> Result<()> {
        let whitelist = &mut self.whitelist;
        whitelist.address = Vec::new();
        whitelist.amount = Vec::new();
        whitelist.bump = bump;

        Ok(())
    }

    fn initialize_transfer_hook(&mut self) -> Result<()> {

        msg!("Initializing Transfer Hook...");

        // Get the extra account metas for the transfer hook
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        msg!("Extra Account Metas: {:?}", extra_account_metas);
        msg!("Extra Account Metas Length: {}", extra_account_metas.len());

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut self.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas,
        )
        .unwrap();

        Ok(())
    }
}
