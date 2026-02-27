use pinocchio::error::ProgramError;

pub mod initialize;
pub use initialize::*;

pub enum FundraiserInstructions {
    Initialize = 0,
}

impl TryFrom<&u8> for FundraiserInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstructions::Initialize),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
