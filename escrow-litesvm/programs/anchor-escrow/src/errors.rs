use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Take is not allowed before the unlock time")]
    TakeBeforeUnlock,
}