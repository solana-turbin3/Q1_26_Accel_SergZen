use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_pubkey::derive_address;
use pinocchio_token::state::{Mint, TokenAccount};

use crate::state::Escrow;

pub fn process_take_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [
        taker, 
        maker, 
        mint_a, 
        mint_b, 
        taker_ata_a, 
        taker_ata_b, 
        maker_ata_b, 
        escrow_account, 
        escrow_ata, 
        system_program, 
        token_program, 
        _remaining_accounts @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !taker.is_signer() {
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
        let taker_ata_b_state = TokenAccount::from_account_view(&taker_ata_b)?;
        if taker_ata_b_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        if taker_ata_a.data_len() == 0 {
            Create {
                funding_account: taker,
                account: taker_ata_a,
                wallet: taker,
                mint: mint_a,
                token_program,
                system_program,
            }
            .invoke()?;
        } else {
            let taker_ata_a_state = TokenAccount::from_account_view(taker_ata_a)?;
            if taker_ata_a_state.mint() != mint_a.address() {
                return Err(ProgramError::InvalidAccountData);
            }
            if taker_ata_a_state.owner() != taker.address() {
                return Err(ProgramError::IllegalOwner);
            }
        }

        if maker_ata_b.data_len() == 0 {
            Create {
                funding_account: taker,
                account: maker_ata_b,
                wallet: maker,
                mint: mint_b,
                token_program,
                system_program,
            }
            .invoke()?;
        } else {
            let maker_ata_b_state = TokenAccount::from_account_view(maker_ata_b)?;
            if maker_ata_b_state.mint() != mint_b.address() {
                return Err(ProgramError::InvalidAccountData);
            }
            if maker_ata_b_state.owner() != maker.address() {
                return Err(ProgramError::IllegalOwner);
            }
        }
    }

    let (amount_to_give, amount_to_receive, bump_bytes, escrow_seed_bytes) = {
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
            || escrow_state.mint_b() != *mint_b.address()
            || escrow_account_pda != *escrow_account.address().as_array()
        {
            return Err(ProgramError::InvalidAccountData);
        }

        let escrow_ata_state = TokenAccount::from_account_view(&escrow_ata)?;
        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }

        let amount_to_give = escrow_state.amount_to_give();
        let amount_to_receive = escrow_state.amount_to_receive();
        let bump_bytes = [escrow_state.bump];
        let escrow_seed_bytes = escrow_state.seed().to_le_bytes();

        (
            amount_to_give,
            amount_to_receive,
            bump_bytes,
            escrow_seed_bytes,
        )
    };

    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
    }
    .invoke()?;

    let vault_seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&escrow_seed_bytes),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&vault_seed);

    pinocchio_token::instructions::Transfer {
        from: escrow_ata,
        to: taker_ata_a,
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
