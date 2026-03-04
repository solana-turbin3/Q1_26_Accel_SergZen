use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

use crate::state::Escrow;

use wincode::SchemaRead;

#[derive(SchemaRead)]
pub struct MakeInstructionData {
    pub bump: u8,
    pub amount_to_receive: u64,
    pub amount_to_give: u64,
    pub seed: u64,
}

pub fn process_make_instruction_v2(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker, 
        mint_a, 
        mint_b, 
        escrow_account, 
        maker_ata, 
        escrow_ata, 
        system_program, 
        token_program, 
        _remaining_accounts @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if system_program.address() != &pinocchio_system::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if token_program.address() != &pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let _mint_a_state = Mint::from_account_view(mint_a)?;
    let _mint_b_state = Mint::from_account_view(mint_b)?;
    unsafe {
        if mint_a.owner() != token_program.address() || mint_b.owner() != token_program.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(&maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let ix_data: MakeInstructionData =
        wincode::deserialize(data).map_err(|_| ProgramError::InvalidInstructionData)?;

    let escrow_bump = [ix_data.bump];
    let amount_to_receive = ix_data.amount_to_receive;
    let amount_to_give = ix_data.amount_to_give;

    let escrow_seed = ix_data.seed;
    let escrow_seed_bytes = escrow_seed.to_le_bytes();

    let seed = [
        b"escrow",
        maker.address().as_ref(),
        &escrow_seed_bytes,
        &escrow_bump,
    ];
    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());

    if escrow_account_pda != *escrow_account.address().as_array() {
        return Err(ProgramError::InvalidAccountData);
    }

    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&escrow_seed_bytes),
        Seed::from(&escrow_bump),
    ];
    let seeds = Signer::from(&seed);

    unsafe {
        if escrow_account.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: escrow_account,
                lamports: Rent::get()?.try_minimum_balance(Escrow::LEN)?,
                space: Escrow::LEN as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[seeds])?;

            {
                let escrow_state = Escrow::from_account_info(escrow_account)?;

                escrow_state.set_maker(maker.address());
                escrow_state.set_mint_a(mint_a.address());
                escrow_state.set_mint_b(mint_b.address());
                escrow_state.set_amount_to_receive(amount_to_receive);
                escrow_state.set_amount_to_give(amount_to_give);
                escrow_state.set_seed(escrow_seed);
                escrow_state.bump = ix_data.bump;
            }
        } else {
            return Err(ProgramError::IllegalOwner);
        }
    }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: escrow_ata,
        wallet: escrow_account,
        mint: mint_a,
        token_program: token_program,
        system_program: system_program,
    }
    .invoke()?;

    pinocchio_token::instructions::Transfer {
        from: maker_ata,
        to: escrow_ata,
        authority: maker,
        amount: amount_to_give,
    }
    .invoke()?;

    Ok(())
}
