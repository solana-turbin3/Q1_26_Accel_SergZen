use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_pubkey::derive_address;

use pinocchio_token::{instructions::{CloseAccount, Transfer}, state::{Mint, TokenAccount}};

use crate::{error::FundraiserError, state::fundraiser::Fundraiser};

pub fn process_checker_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint,
        fundraiser,
        vault,
        maker_ata,
        token_program,
        system_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if token_program.address() != &pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if system_program.address() != &pinocchio_system::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    {
        if maker_ata.data_len() == 0 {
            Create {
                funding_account: maker,
                account: maker_ata,
                wallet: maker,
                mint: mint,
                token_program,
                system_program,
            }
            .invoke()?;
        } else {
            let maker_ata_state = TokenAccount::from_account_view(maker_ata)?;
            if maker_ata_state.mint() != mint.address() {
                return Err(ProgramError::InvalidAccountData);
            }
            if maker_ata_state.owner() != maker.address() {
                return Err(ProgramError::IllegalOwner);
            }
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

    {
        let vault_state = TokenAccount::from_account_view(&vault)?;
        if vault_state.owner() != fundraiser.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if vault_state.mint() != mint.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        if vault_state.amount() < fundraiser_state.amount_to_raise() {
            return Err(FundraiserError::TargetNotMet.into());
        }
    }

    let fundraiser_bump = [fundraiser_state.bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&fundraiser_bump),
    ];
    let fundraiser_signer = Signer::from(&signer_seeds);

    Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: fundraiser_state.current_amount(),
    }
    .invoke_signed(&[fundraiser_signer.clone()])?;

    CloseAccount {
        account: vault,
        destination: maker,
        authority: fundraiser,
    }
    .invoke_signed(&[fundraiser_signer])?;

    let fundraiser_lamports = fundraiser.lamports();
    maker.set_lamports(
        maker.lamports()
        .checked_add(fundraiser_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?
    );
    fundraiser.set_lamports(0);

    fundraiser.close()?;
    
    Ok(())
}