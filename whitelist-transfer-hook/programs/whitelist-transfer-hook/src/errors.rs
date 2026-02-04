use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistTransferHookError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Extra Account Meta Error")]
    ExtraAccountMetaError,
}