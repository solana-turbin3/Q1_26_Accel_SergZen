use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    #[max_len(1024)]
    pub address: Vec<Pubkey>,
    #[max_len(1024)]
    pub amount: Vec<u64>,
    pub bump: u8,
}

impl Whitelist {
    pub fn get_index(&self, key: Pubkey) -> Option<usize> {
        self.address.iter().position(|address| *address == key)
    }
}
