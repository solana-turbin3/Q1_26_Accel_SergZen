use pinocchio::error::ProgramError;

#[derive(Debug)]
pub enum FundraiserError {
    TargetNotMet,
    TargetMet,
    ContributionTooBig,
    ContributionTooSmall,
    MaximumContributionsReached,
    FundraiserNotEnded,
    FundraiserEnded,
    InvalidAmount,
}

impl From<FundraiserError> for ProgramError {
    fn from(e: FundraiserError) -> Self {
        ProgramError::Custom(e as u32)
    }
}