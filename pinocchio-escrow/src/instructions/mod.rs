pub mod cancel;
pub mod cancel_v2;
pub mod make;
pub mod make_v2;
pub mod take;
pub mod take_v2;

pub use cancel::*;
pub use cancel_v2::*;
pub use make::*;
pub use make_v2::*;
pub use take::*;
pub use take_v2::*;

use pinocchio::error::ProgramError;

#[repr(u8)]
pub enum EscrowInstructions {
    Make = 0,
    Take = 1,
    Cancel = 2,
    MakeV2 = 3,
    TakeV2 = 4,
    CancelV2 = 5,
}

impl TryFrom<&u8> for EscrowInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EscrowInstructions::Make),
            1 => Ok(EscrowInstructions::Take),
            2 => Ok(EscrowInstructions::Cancel),
            3 => Ok(EscrowInstructions::MakeV2),
            4 => Ok(EscrowInstructions::TakeV2),
            5 => Ok(EscrowInstructions::CancelV2),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
