use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserDeposit {
    pub address: Pubkey,
    pub amount: u64,
    pub bump: u8,
}