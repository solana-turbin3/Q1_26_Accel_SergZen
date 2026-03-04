#![allow(unexpected_cfgs)]
use pinocchio::{
    AccountView, Address, ProgramResult, address::declare_id, entrypoint, error::ProgramError,
};

use crate::instructions::FundraiserInstructions;

mod constants;
mod instructions;
mod state;
mod tests;
mod error;

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
        FundraiserInstructions::Initialize => instructions::process_initialize_instruction(accounts, data),
        FundraiserInstructions::Contribute => instructions::process_contribute_instruction(accounts, data),
        FundraiserInstructions::Checker => instructions::process_checker_instruction(accounts, data),
        FundraiserInstructions::Refund => instructions::process_refund_instruction(accounts, data),
//      _ => return Err(ProgramError::InvalidInstructionData),
    }
}
