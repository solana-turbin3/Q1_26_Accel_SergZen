use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::{CreateAccount};
use pinocchio_token::{instructions::Transfer, state::{Mint, TokenAccount}};
use wincode::SchemaRead;

use crate::{
    constants::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS}, 
    error::FundraiserError, 
    state::{contributor::Contributor, fundraiser::Fundraiser}
};

#[derive(SchemaRead)]
struct ContributeData {
    pub bump: u8,
    pub amount: [u8; 8],
}

pub fn process_contribute_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [
        contributor,
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

        let vault_state = TokenAccount::from_account_view(&vault)?;
        if vault_state.owner() != fundraiser.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if vault_state.mint() != mint.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let mint_state = Mint::from_account_view(mint)?;
    unsafe {
        if mint.owner() != token_program.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let ix_data = ::wincode::deserialize::<ContributeData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let contributor_bump = [ix_data.bump];
    let signer_seeds = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser.address().as_ref()),
        Seed::from(contributor.address().as_ref()),
        Seed::from(&contributor_bump),
    ];
    let contributor_signer = Signer::from(&signer_seeds);

    let contributor_state = {
        if contributor_account.data_len() == 0 {
            CreateAccount {
                from: contributor,
                to: contributor_account,
                space: Contributor::LEN as u64,
                owner: &crate::ID,
                lamports: Rent::get()?.minimum_balance_unchecked(Contributor::LEN),
            }
            .invoke_signed(&[contributor_signer])?;

            let contributor_state = Contributor::from_account_info(contributor_account)?;
            contributor_state.bump = ix_data.bump;

            contributor_state
        } else {
           Contributor::from_account_info(contributor_account)?
        }
    };

    let fundraiser_state = {
        let fundraiser_state = Fundraiser::from_account_info(fundraiser)?;
        let seeds = [
            b"fundraiser",
            fundraiser_state.maker.as_ref(),
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

    let amount = u64::from_le_bytes(ix_data.amount);

    // Check if the amount to contribute meets the minimum amount required
    if amount < 10_u8.pow(mint_state.decimals() as u32) as u64 {
        return Err(FundraiserError::ContributionTooSmall.into());
    }
    
    // Check if the amount to contribute is less than the maximum allowed contribution
    if amount > (fundraiser_state.amount_to_raise() * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER {
        return Err(FundraiserError::ContributionTooBig.into());
    } 

    // Check if the fundraising duration has been reached
    let current_time = Clock::get()?.unix_timestamp;
    if fundraiser_state.duration > ((current_time - fundraiser_state.time_started()) / SECONDS_TO_DAYS) as u8 {
        return Err(FundraiserError::FundraiserEnded.into());
    }

    // Check if the maximum contributions per contributor have been reached
    if contributor_state.amount() + amount > fundraiser_state.max_contribution() {
        return Err(FundraiserError::MaximumContributionsReached.into());   
    }

    Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount: amount,
    }
    .invoke()?;

    fundraiser_state.add_current_amount(amount)?;
    contributor_state.add_amount(amount)?;
    
    Ok(())
}