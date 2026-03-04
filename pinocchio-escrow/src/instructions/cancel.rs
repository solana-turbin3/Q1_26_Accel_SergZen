use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::state::{Mint, TokenAccount};

use crate::state::Escrow;

pub fn process_cancel_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [
        maker, 
        mint_a, 
        maker_ata_a, 
        escrow_account, 
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
    
    if token_program.address() != &pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if system_program.address() != &pinocchio_system::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let _mint_a_state = Mint::from_account_view(mint_a)?;
    unsafe {
        if mint_a.owner() != token_program.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    {
        let maker_ata_a_state = TokenAccount::from_account_view(maker_ata_a)?;
        if maker_ata_a_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if maker_ata_a_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let (amount_to_give, bump_bytes, escrow_seed_bytes) = {
        let escrow_state = Escrow::from_account_info(escrow_account)?;
        let seeds = [
            b"escrow",
            maker.address().as_ref(),
            &escrow_state.seed().to_le_bytes(),
            &[escrow_state.bump],
        ];
        let escrow_account_pda = derive_address(&seeds, None, &crate::ID.as_array());

        if escrow_state.maker() != *maker.address()
            || escrow_state.mint_a() != *mint_a.address()
            || escrow_account_pda != *escrow_account.address().as_array()
        {
            return Err(ProgramError::InvalidAccountData);
        }

        let escrow_ata_state = TokenAccount::from_account_view(&escrow_ata)?;
        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }

        let amount_to_give = escrow_state.amount_to_give();
        let bump_bytes = [escrow_state.bump];
        let escrow_seed_bytes = escrow_state.seed().to_le_bytes();

        (amount_to_give, bump_bytes, escrow_seed_bytes)
    };

    let vault_seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&escrow_seed_bytes),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&vault_seed);

    pinocchio_token::instructions::Transfer {
        from: escrow_ata,
        to: maker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[signer.clone()])?;

    pinocchio_token::instructions::CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer])?;

    let escrow_lamports = escrow_account.lamports();
    escrow_account.set_lamports(0);
    maker.set_lamports(
        maker
            .lamports()
            .checked_add(escrow_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );

    escrow_account.close()?;

    Ok(())
}
