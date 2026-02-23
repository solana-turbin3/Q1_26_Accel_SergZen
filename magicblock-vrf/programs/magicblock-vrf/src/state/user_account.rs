use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    pub user: Pubkey,
    pub data: u64,
    pub random: u64,
    pub bump: u8,
}