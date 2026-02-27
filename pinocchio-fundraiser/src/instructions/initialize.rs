use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use wincode::SchemaRead;

use crate::{constants::MIN_AMOUNT_TO_RAISE, state::fundraiser::Fundraiser};

// use crate::state::Escrow;
#[derive(SchemaRead)]
struct InitializeData {
    pub bump: u8,
    pub amount_to_raise: [u8; 8],
    pub duration: u8,
}

pub fn process_initialize_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint,
        fundraiser,
        vault,
        system_program,
        token_program,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let ix_data = ::wincode::deserialize::<InitializeData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let mint_state = pinocchio_token::state::Mint::from_account_view(mint)?;

    assert!(
        u64::from_le_bytes(ix_data.amount_to_raise)
            > MIN_AMOUNT_TO_RAISE.pow(mint_state.decimals() as u32),
        "amount to be raised is less than minimum amount"
    );

    let bump = ix_data.bump;
    let seed = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];

    let fundraiser_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(fundraiser_account_pda, *fundraiser.address().as_array());

    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);

    unsafe {
        if fundraiser.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: fundraiser,
                lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
                space: Fundraiser::LEN as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[seeds.clone()])?;

            {
                let fundraiser_state = Fundraiser::from_account_info(fundraiser)?;

                fundraiser_state.maker = *maker.address().as_array();
                fundraiser_state.mint_to_raise = *mint.address().as_array();
                fundraiser_state.amount_to_raise = ix_data.amount_to_raise;
                fundraiser_state.current_amount = [0; 8];
                fundraiser_state.time_started = Clock::get()?.unix_timestamp.to_le_bytes(); // ???
                fundraiser_state.duration = ix_data.duration; // ???
                fundraiser_state.bump = ix_data.bump;
            }
        } else {
            return Err(ProgramError::IllegalOwner);
        }
    }

    // we can do this client side to reduce CU
    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint,
        token_program: token_program,
        system_program: system_program,
    }
    .invoke()?;

    Ok(())
}
