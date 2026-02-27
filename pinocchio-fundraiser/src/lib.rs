#![allow(unexpected_cfgs)]
use pinocchio::{
    AccountView, Address, ProgramResult, address::declare_id, entrypoint, error::ProgramError,
};

use crate::instructions::FundraiserInstructions;

// use crate::instructions::EscrowInstrctions;

mod constants;
mod instructions;
mod state;

entrypoint!(process_instruction);

declare_id!("4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT");

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundraiserInstructions::try_from(discriminator)? {
        FundraiserInstructions::Initialize => {
            instructions::process_initialize_instruction(accounts, data)?
        } // _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
