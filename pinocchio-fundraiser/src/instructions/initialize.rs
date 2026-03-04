use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use wincode::SchemaRead;

use crate::{
    constants::MIN_AMOUNT_TO_RAISE, 
    error::FundraiserError, state::fundraiser::Fundraiser
};

#[derive(SchemaRead)]
struct InitializeData {
    pub bump: u8,
    pub amount: [u8; 8],
    pub duration: u8,
}

pub fn process_initialize_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint,
        fundraiser,
        vault,
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

    let mint_state = pinocchio_token::state::Mint::from_account_view(mint)?;
    unsafe {
        if mint.owner() != token_program.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let ix_data = ::wincode::deserialize::<InitializeData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    if u64::from_le_bytes(ix_data.amount) < MIN_AMOUNT_TO_RAISE.pow(mint_state.decimals() as u32) {
        return Err(FundraiserError::InvalidAmount.into());
    }

    let fundraiser_bump = &[ix_data.bump];
    let seed = [
        b"fundraiser", 
        maker.address().as_ref(), 
        fundraiser_bump
    ];

    let fundraiser_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    if fundraiser_account_pda != *fundraiser.address().as_array() {
        return Err(ProgramError::InvalidAccountData);
    }    

    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(fundraiser_bump),
    ];
    let fundraiser_signer = Signer::from(&signer_seeds);

    unsafe {
        if fundraiser.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: fundraiser,
                lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
                space: Fundraiser::LEN as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[fundraiser_signer])?;

            {
                let fundraiser_state = Fundraiser::from_account_info(fundraiser)?;

                fundraiser_state.maker = *maker.address().as_array();
                fundraiser_state.mint_to_raise = *mint.address().as_array();
                fundraiser_state.amount_to_raise = ix_data.amount;
                fundraiser_state.time_started = Clock::get()?.unix_timestamp.to_le_bytes();
                fundraiser_state.duration = ix_data.duration;
                fundraiser_state.bump = ix_data.bump;
            }
        } else {
            return Err(ProgramError::IllegalOwner);
        }
    }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint,
        token_program: token_program,
        system_program: system_program,
    }
    .invoke()
}
