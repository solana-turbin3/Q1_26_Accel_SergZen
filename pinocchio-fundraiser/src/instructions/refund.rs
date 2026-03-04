use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::{instructions::Transfer, state::{Mint, TokenAccount}};

use crate::{constants::SECONDS_TO_DAYS, error::FundraiserError, state::{contributor::Contributor, fundraiser::Fundraiser}};

pub fn process_refund_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [
        contributor,
        maker,
        mint,
        fundraiser,
        vault,
        contributor_account,
        contributor_ata,
        token_program,
        system_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if token_program.address() != &pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if system_program.address() != &pinocchio_system::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    {
        let contributor_ata_state = TokenAccount::from_account_view(&contributor_ata)?;
        if contributor_ata_state.owner() != contributor.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if contributor_ata_state.mint() != mint.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let _mint_state = Mint::from_account_view(mint)?;
    unsafe {
        if mint.owner() != token_program.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let fundraiser_state = {
        let fundraiser_state = Fundraiser::from_account_info(fundraiser)?;
        let seeds = [
            b"fundraiser",
            maker.address().as_ref(),
            &[fundraiser_state.bump],
        ];
        let fundraiser_account_pda = derive_address(&seeds, None, &crate::ID.as_array());

        if fundraiser_state.mint_to_raise != *mint.address().as_ref()
            || fundraiser_account_pda != *fundraiser.address().as_ref()
        {
            return Err(ProgramError::InvalidAccountData);
        }

        fundraiser_state
    };

    let contributor_state = {
        let contributor_state = Contributor::from_account_info(contributor_account)?;

        let seeds:[&[u8]; 4] = [
            b"contributor",
            fundraiser.address().as_ref(),
            contributor.address().as_ref(),
            &[contributor_state.bump],
        ];
        let contributor_account_pda = derive_address(&seeds, None, &crate::ID.as_array());

        if contributor_account_pda != *contributor_account.address().as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }

        contributor_state
    };

    {
        let vault_state = TokenAccount::from_account_view(&vault)?;
        if vault_state.owner() != fundraiser.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if vault_state.mint() != mint.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        if vault_state.amount() >= fundraiser_state.amount_to_raise() {
            return Err(FundraiserError::TargetMet.into());
        }  
    }

    // Check if the fundraising duration has been reached
    let current_time = Clock::get()?.unix_timestamp;
    if fundraiser_state.duration < ((current_time - fundraiser_state.time_started()) / SECONDS_TO_DAYS) as u8 {
        return Err(FundraiserError::FundraiserNotEnded.into());
    }

    let refund_amount = contributor_state.amount();

    let fundraiser_bump = [fundraiser_state.bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&fundraiser_bump),
    ];
    let fundraiser_signer = Signer::from(&signer_seeds);

    Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: refund_amount,
    }
    .invoke_signed(&[fundraiser_signer])?;

    fundraiser_state.sub_current_amount(refund_amount)?;

    let contributor_account_lamports = contributor_account.lamports();
    contributor.set_lamports(
        contributor.lamports()
        .checked_add(contributor_account_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?
    );
    contributor_account.set_lamports(0);

    contributor_account.close()?;
    
    Ok(())
}