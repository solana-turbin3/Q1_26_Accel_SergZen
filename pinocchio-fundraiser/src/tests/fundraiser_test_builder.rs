#![cfg(test)]
use {
    crate::{instructions::FundraiserInstructions, state::fundraiser::Fundraiser},
    litesvm::{
        LiteSVM,
        types::{FailedTransactionMetadata, TransactionMetadata},
    },
    litesvm_token::{
        CreateAssociatedTokenAccount, CreateMint, MintTo, spl_token::ID as TOKEN_PROGRAM_ID,
    },
    pinocchio_token::state::TokenAccount,
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_message::{AccountMeta, Message},
    solana_native_token::LAMPORTS_PER_SOL,
    solana_pubkey::Pubkey,
    solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
    solana_signer::Signer,
    solana_transaction::Transaction,
    spl_associated_token_account::{
        ID as ASSOCIATED_TOKEN_PROGRAM_ID, get_associated_token_address,
    },
    std::path::PathBuf,
    wincode::SchemaWrite,
};

const PROGRAM_ID: Pubkey = crate::ID;

#[derive(SchemaWrite)]
struct InitializeData {
    pub bump: u8,
    pub amount: u64,
    pub duration: u8,
}

#[derive(SchemaWrite)]
struct ContributeInstructionData {
    pub bump: u8,
    pub amount: u64,
}

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

pub struct FundraiserTestBuilder {
    svm: LiteSVM,
    maker: Keypair,
    maker_ata: Option<Pubkey>,
    mint: Option<Pubkey>,
    fundraiser: Option<(Pubkey, u8)>,
    vault: Option<Pubkey>,
    last_tx: Option<TransactionMetadata>,
    last_tx_error: Option<String>,
}

impl FundraiserTestBuilder {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let maker = Keypair::new();

        svm.airdrop(&maker.pubkey(), 30 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to maker");

        println!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));
        let so_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/deploy/pinocchio_fundraiser.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(PROGRAM_ID, &program_data)
            .expect("Failed to add program");

        Self {
            svm,
            maker,
            maker_ata: None,
            mint: None,
            fundraiser: None,
            vault: None,
            last_tx: None,
            last_tx_error: None,
        }
    }

    pub fn create_mint(mut self) -> Self {
        let mint = CreateMint::new(&mut self.svm, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();
        println!("Mint: {}", mint);

        self.mint = Some(mint);

        self
    }

    pub fn create_maker_ata(mut self) -> Self {
        let mint = self.mint.expect("Mint A not created");
        let maker_ata = CreateAssociatedTokenAccount::new(&mut self.svm, &self.maker, &mint)
            .owner(&self.maker.pubkey())
            .send()
            .unwrap();
        println!("Maker ATA: {}\n", maker_ata);

        self.maker_ata = Some(maker_ata);

        self
    }

    pub fn setup_contributor(mut self, contributor: &Keypair, amount: u64) -> Self {
        self.svm
            .airdrop(&contributor.pubkey(), 20 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to contributor");

        let contributor_ata =
            CreateAssociatedTokenAccount::new(&mut self.svm, &contributor, &self.mint.unwrap())
                .owner(&contributor.pubkey())
                .send()
                .unwrap();

        MintTo::new(
            &mut self.svm,
            &self.maker,
            &self.mint.unwrap(),
            &contributor_ata,
            amount,
        )
        .send()
        .unwrap();

        self
    }

    pub fn execute_initialize(mut self, amount: u64, duration: u8) -> Self {
        let fundraiser = Pubkey::find_program_address(
            &[b"fundraiser".as_ref(), self.maker.pubkey().as_ref()],
            &PROGRAM_ID,
        );
        self.fundraiser = Some(fundraiser);

        println!("Fundraiser PDA: {}\n", self.fundraiser_pubkey());

        let vault = spl_associated_token_account::get_associated_token_address(
            &self.fundraiser_pubkey(),
            &self.mint.unwrap(),
        );
        println!("Vault: {}\n", vault);
        self.vault = Some(vault);

        let bump: u8 = self.fundraiser_bump();
        println!("Bump: {}", bump);

        let ix_data = InitializeData {
            bump,
            amount,
            duration,
        };
        let encoded = wincode::serialize(&ix_data).unwrap();
        let initialize_data = [vec![FundraiserInstructions::Initialize as u8], encoded].concat();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;

        let initialize_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint.unwrap(), false),
                AccountMeta::new(self.fundraiser_pubkey(), false),
                AccountMeta::new(self.vault.unwrap(), false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: initialize_data,
        };

        let tx = send_tx(&mut self.svm, &[initialize_ix], &self.maker, &[&self.maker]).unwrap();

        println!("\n\nInitialize transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        println!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;

        self
    }

    pub fn execute_contribute(mut self, contributor: &Keypair, amount: u64) -> Self {
        let contributor_ata = self.get_contributor_ata(&contributor.pubkey());
        let contributor_pda = self.get_contributor_pda(&contributor.pubkey());

        let bump: u8 = contributor_pda.1;
        println!("Bump: {}", bump);

        let ix_data = ContributeInstructionData { bump, amount };
        let encoded = wincode::serialize(&ix_data).unwrap();
        let contribute_data = [vec![FundraiserInstructions::Contribute as u8], encoded].concat();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;

        let contribute_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(self.mint.unwrap(), false),
                AccountMeta::new(self.fundraiser_pubkey(), false),
                AccountMeta::new(self.vault.unwrap(), false),
                AccountMeta::new(contributor_pda.0, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: contribute_data,
        };

        let tx = send_tx(
            &mut self.svm,
            &[contribute_ix],
            &contributor,
            &[&contributor],
        )
        .unwrap();

        println!("\n\nContribute transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        println!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;

        self
    }

    pub fn execute_refund(mut self, contributor: &Keypair) -> Self {
        let contributor_ata = self.get_contributor_ata(&contributor.pubkey());
        let contributor_pda = self.get_contributor_pda(&contributor.pubkey());

        let refund_data = [vec![FundraiserInstructions::Refund as u8]].concat();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;

        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(self.maker.pubkey(), false),
                AccountMeta::new_readonly(self.mint.unwrap(), false),
                AccountMeta::new(self.fundraiser_pubkey(), false),
                AccountMeta::new(self.vault.unwrap(), false),
                AccountMeta::new(contributor_pda.0, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: refund_data,
        };

        let tx = send_tx(&mut self.svm, &[refund_ix], &contributor, &[&contributor]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nRefund transaction successful");
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

    pub fn execute_checker(mut self) -> Self {
        let checker_data = [vec![FundraiserInstructions::Checker as u8]].concat();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID;

        let checker_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.maker.pubkey(), true),
                AccountMeta::new_readonly(self.mint.unwrap(), false),
                AccountMeta::new(self.fundraiser_pubkey(), false),
                AccountMeta::new(self.vault.unwrap(), false),
                AccountMeta::new(self.maker_ata.unwrap(), false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: checker_data,
        };

        let tx = send_tx(&mut self.svm, &[checker_ix], &self.maker, &[&self.maker]);

        match &tx {
            Ok(tx_result) => {
                println!("\n\nChecker transaction successful");
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

    pub fn get_contributor_ata(&self, contributor: &Pubkey) -> Pubkey {
        get_associated_token_address(&contributor, &self.mint.unwrap())
    }

    pub fn get_contributor_pda(&self, contributor: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"contributor",
                self.fundraiser_pubkey().as_ref(),
                contributor.as_ref(),
            ],
            &PROGRAM_ID,
        )
    }

    pub fn contributor_ata_data(&self, contributor: &Pubkey) -> TokenAccount {
        let contributor_ata = self.get_contributor_ata(contributor);
        let account = self.svm.get_account(&contributor_ata).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn is_contributor_closed(&self, contributor: &Pubkey) -> bool {
        let contributor_pda = self.get_contributor_pda(&contributor);
        self.svm.get_account(&contributor_pda.0).is_none()
    }

    pub fn maker_ata_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.maker_ata.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn maker_pubkey(&self) -> Pubkey {
        self.maker.pubkey()
    }

    pub fn mint(&self) -> Pubkey {
        self.mint.unwrap()
    }

    pub fn fundraiser_pubkey(&self) -> Pubkey {
        self.fundraiser.unwrap().0
    }

    pub fn fundraiser_bump(&self) -> u8 {
        self.fundraiser.unwrap().1
    }

    pub fn fundraiser_data(&self) -> Fundraiser {
        let fundraiser_account = self.svm.get_account(&self.fundraiser.unwrap().0).unwrap();
        let data = &fundraiser_account.data;
        unsafe { std::ptr::read(data.as_ptr() as *const Fundraiser) }
    }

    pub fn is_fundraiser_closed(&self) -> bool {
        self.svm.get_account(&self.fundraiser.unwrap().0).is_none()
    }

    pub fn vault_data(&self) -> TokenAccount {
        let account = self.svm.get_account(&self.vault.unwrap()).unwrap();
        unsafe { std::ptr::read(account.data.as_ptr() as *const TokenAccount) }
    }

    pub fn is_vault_ata_closed(&self) -> bool {
        self.svm.get_account(&self.vault.unwrap()).is_none()
    }

    pub fn last_tx_succeeded(&self) -> bool {
        self.last_tx_error.is_none()
    }
}
