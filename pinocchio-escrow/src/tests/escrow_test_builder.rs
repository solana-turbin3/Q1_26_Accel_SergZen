#![cfg(test)]
use {
    crate::instructions::EscrowInstructions, litesvm::{
        LiteSVM, types::{FailedTransactionMetadata, TransactionMetadata}
    }, litesvm_token::{
        CreateAssociatedTokenAccount, CreateMint, MintTo, spl_token::ID as TOKEN_PROGRAM_ID
    }, pinocchio_token::state::TokenAccount, solana_instruction::Instruction, solana_keypair::Keypair, solana_message::{AccountMeta, Message}, solana_native_token::LAMPORTS_PER_SOL, solana_pubkey::Pubkey, solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID, solana_signer::Signer, solana_transaction::Transaction, spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID, std::path::PathBuf, wincode::{SchemaRead, SchemaWrite}
};

const PROGRAM_ID: Pubkey = crate::ID;

fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let message = Message::new(ixs, Some(&payer.pubkey()));
    let recent_blockhash = svm.latest_blockhash();
    let transaction: Transaction = Transaction::new(signers, message, recent_blockhash);
    let tx = svm.send_transaction(transaction);

    tx
}

pub struct EscrowTestBuilder {
    svm: LiteSVM,
    maker: Keypair,
    taker: Option<Keypair>,
    mint_a: Option<Pubkey>,
    mint_b: Option<Pubkey>,
    maker_ata_a: Option<Pubkey>,
    maker_ata_b: Option<Pubkey>,
    taker_ata_a: Option<Pubkey>,
    taker_ata_b: Option<Pubkey>,
    escrow: Option<(Pubkey, u8)>,
    escrow_ata: Option<Pubkey>,
    last_tx: Option<TransactionMetadata>,
    last_tx_error: Option<String>,
}

impl EscrowTestBuilder {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let maker = Keypair::new();

        svm.airdrop(&maker.pubkey(), 30 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to maker");

        println!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/deploy/escrow.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(PROGRAM_ID, &program_data)
            .expect("Failed to add program");

        Self {
            svm,
            maker,
            taker: None,
            mint_a: None,
            mint_b: None,
            maker_ata_a: None,
            maker_ata_b: None,
            taker_ata_a: None,
            taker_ata_b: None,
            escrow: None,
            escrow_ata: None,
            last_tx: None,
            last_tx_error: None,
        }
    }

    pub fn create_mints(mut self) -> Self {
        let mint_a = CreateMint::new(&mut self.svm, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();
        println!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut self.svm, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();
        println!("Mint B: {}", mint_b);

        self.mint_a = Some(mint_a);
        self.mint_b = Some(mint_b);

        self
    }

    pub fn create_maker_ata_a(mut self) -> Self {
        let mint_a = self.mint_a.expect("Mint A not created");
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut self.svm, &self.maker, &mint_a)
            .owner(&self.maker.pubkey())
            .send()
            .unwrap();
        println!("Maker ATA A: {}\n", maker_ata_a);

        self.maker_ata_a = Some(maker_ata_a);

        self
    }

    pub fn create_maker_ata_b(mut self) -> Self {
        let mint_b = self.mint_b.expect("Mint B not created");
        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut self.svm, &self.maker, &mint_b)
            .owner(&self.maker.pubkey())
            .send()
            .unwrap();

        self.maker_ata_b = Some(maker_ata_b);

        self
    }

    pub fn mint_to_maker_ata_a(mut self, amount: u64) -> Self {
        MintTo::new(
            &mut self.svm,
            &self.maker,
            &self.mint_a.unwrap(),
            &self.maker_ata_a.unwrap(),
            amount,
        )
        .send()
        .unwrap();

        self
    }

    pub fn mint_to_taker_ata_b(mut self, amount: u64) -> Self {
        MintTo::new(
            &mut self.svm,
            &self.maker,
            &self.mint_b.unwrap(),
            &self.taker_ata_b.unwrap(),
            amount,
        )
        .send()
        .unwrap();

        self
    }

    pub fn setup_taker(mut self) -> Self {
        let taker = Keypair::new();
        self.svm
            .airdrop(&taker.pubkey(), 20 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to taker");

        self.taker = Some(taker);

        self
    }

    pub fn create_taker_atas(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");
        let mint_a = self.mint_a.expect("Mint A not created");
        let mint_b = self.mint_b.expect("Mint B not created");

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut self.svm, taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut self.svm, taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        self.taker_ata_a = Some(taker_ata_a);
        self.taker_ata_b = Some(taker_ata_b);

        self
    }

    pub fn set_escrow_accounts(mut self, seed: u64) -> Self {
        let escrow = Pubkey::find_program_address(
            &[
                b"escrow",
                self.maker.pubkey().as_ref(),
                &seed.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        println!("Escrow PDA: {}\n", escrow.0);

        let escrow_ata = spl_associated_token_account::get_associated_token_address(
            &escrow.0,
            &self.mint_a.unwrap(),
        );
        println!("Escrow ATA: {}\n", escrow_ata);

        self.escrow = Some(escrow);
        self.escrow_ata = Some(escrow_ata);

        self
    }

    pub fn execute_make(mut self, amount_to_give: u64, seed: u64, amount_to_receive: u64) -> Self {
        let bump: u8 = self.escrow_bump();
        println!("Bump: {}", bump);

        let make_data = [
            vec![EscrowInstructions::Make as u8],
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
            seed.to_le_bytes().to_vec(),
        ]
        .concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new_readonly(self.mint_b.unwrap(), false),
                AccountMeta::new(self.escrow_pubkey(), false),
                AccountMeta::new(self.maker_ata_a.unwrap(), false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: make_data,
        };

        let tx = send_tx(&mut self.svm, &[make_ix], &self.maker, &[&self.maker]).unwrap();

        println!("\n\nMake transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        println!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;

        self
    }

    pub fn execute_make_v2(
        mut self,
        amount_to_give: u64,
        seed: u64,
        amount_to_receive: u64,
    ) -> Self {
        let bump: u8 = self.escrow_bump();
        println!("Bump: {}", bump);

        #[derive(SchemaWrite, SchemaRead)]
        pub struct MakeInstructionData {
            pub bump: u8,
            pub amount_to_receive: u64,
            pub amount_to_give: u64,
            pub seed: u64,
        }

        let ix_data = MakeInstructionData {
            bump,
            amount_to_receive,
            amount_to_give,
            seed,
        };
        let encoded = wincode::serialize(&ix_data).unwrap();
        let make_v2_data = [vec![EscrowInstructions::MakeV2 as u8], encoded].concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let make_v2_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new_readonly(self.mint_b.unwrap(), false),
                AccountMeta::new(self.escrow_pubkey(), false),
                AccountMeta::new(self.maker_ata_a.unwrap(), false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: make_v2_data,
        };

        let tx = send_tx(&mut self.svm, &[make_v2_ix], &self.maker, &[&self.maker]).unwrap();

        println!("\n\nMakeV2 transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        println!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;

        self
    }

    pub fn execute_take(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");

        let take_data = [vec![EscrowInstructions::Take as u8]].concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(self.maker.pubkey(), false),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new_readonly(self.mint_b.unwrap(), false),
                AccountMeta::new(self.taker_ata_a.unwrap(), false),
                AccountMeta::new(self.taker_ata_b.unwrap(), false),
                AccountMeta::new(self.maker_ata_b.unwrap(), false),
                AccountMeta::new(self.escrow.unwrap().0, false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: take_data,
        };

        let tx = send_tx(&mut self.svm, &[take_ix], &taker, &[&taker]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nTake transaction successful");
                println!("CUs Consumed: {}", tx_result.compute_units_consumed);
                println!("Tx Signature: {}", tx_result.signature);

                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                print!("Error: {:?}", err);
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn execute_take_v2(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");

        let take_v2_data = [vec![EscrowInstructions::TakeV2 as u8]].concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let take_v2_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(self.maker.pubkey(), false),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new_readonly(self.mint_b.unwrap(), false),
                AccountMeta::new(self.taker_ata_a.unwrap(), false),
                AccountMeta::new(self.taker_ata_b.unwrap(), false),
                AccountMeta::new(self.maker_ata_b.unwrap(), false),
                AccountMeta::new(self.escrow.unwrap().0, false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: take_v2_data,
        };

        let tx = send_tx(&mut self.svm, &[take_v2_ix], &taker, &[&taker]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nTakeV2 transaction successful");
                println!("CUs Consumed: {}", tx_result.compute_units_consumed);
                println!("Tx Signature: {}", tx_result.signature);

                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                print!("Error: {:?}", err);
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn execute_cancel(mut self) -> Self {
        let cancel_data = [vec![EscrowInstructions::Cancel as u8]].concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let cancel_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new(self.maker_ata_a.unwrap(), false),
                AccountMeta::new(self.escrow.unwrap().0, false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: cancel_data,
        };

        let tx = send_tx(&mut self.svm, &[cancel_ix], &self.maker, &[&self.maker]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nCancel transaction successful");
                println!("CUs Consumed: {}", tx_result.compute_units_consumed);
                println!("Tx Signature: {}", tx_result.signature);

                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn execute_cancel_v2(mut self) -> Self {
        let cancel_v2_data = [vec![EscrowInstructions::CancelV2 as u8]].concat();

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let cancel_v2_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint_a.unwrap(), false),
                AccountMeta::new(self.maker_ata_a.unwrap(), false),
                AccountMeta::new(self.escrow.unwrap().0, false),
                AccountMeta::new(self.escrow_ata.unwrap(), false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: cancel_v2_data,
        };

        let tx = send_tx(&mut self.svm, &[cancel_v2_ix], &self.maker, &[&self.maker]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nCancelV2 transaction successful");
                println!("CUs Consumed: {}", tx_result.compute_units_consumed);
                println!("Tx Signature: {}", tx_result.signature);

                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn escrow_ata_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.escrow_ata.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn escrow_data(&self) -> crate::state::Escrow {
        let escrow_account = self.svm.get_account(&self.escrow.unwrap().0).unwrap();
        let data = &escrow_account.data;
        unsafe { std::ptr::read(data.as_ptr() as *const crate::state::Escrow) }
    }

    pub fn maker_ata_a_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.maker_ata_a.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn maker_ata_b_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.maker_ata_b.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn taker_ata_a_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.taker_ata_a.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn taker_ata_b_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.taker_ata_b.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn is_escrow_ata_closed(&self) -> bool {
        self.svm.get_account(&self.escrow_ata.unwrap()).is_none()
    }

    pub fn is_escrow_closed(&self) -> bool {
        self.svm.get_account(&self.escrow.unwrap().0).is_none()
    }

    pub fn maker_pubkey(&self) -> Pubkey {
        self.maker.pubkey()
    }

    pub fn mint_a(&self) -> Pubkey {
        self.mint_a.unwrap()
    }

    pub fn mint_b(&self) -> Pubkey {
        self.mint_b.unwrap()
    }

    pub fn escrow_pubkey(&self) -> Pubkey {
        self.escrow.unwrap().0
    }

    pub fn escrow_bump(&self) -> u8 {
        self.escrow.unwrap().1
    }

    pub fn last_tx_succeeded(&self) -> bool {
        self.last_tx_error.is_none()
    }
}
