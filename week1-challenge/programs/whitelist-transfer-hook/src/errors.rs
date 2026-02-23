use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistTransferHookError {
    #[msg("Invalid admin")]
    InvalidAdmin,

    #[msg("Extra Account Meta Error")]
    ExtraAccountMetaError,

    #[msg("Not Transferring")]
    NotTransferring,

    #[msg("Not Whitelisted")]
    NotWhitelisted
}
