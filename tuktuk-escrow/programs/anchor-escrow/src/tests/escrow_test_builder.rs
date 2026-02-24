#![cfg(test)]
use {
    anchor_lang::{
        prelude::msg, solana_program::program_pack::Pack, AccountDeserialize, InstructionData,
        ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, spl_associated_token_account},
        token::spl_token,
    },
    litesvm::{types::TransactionMetadata, LiteSVM},
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_account::Account,
    solana_address::Address,
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
    solana_signer::Signer,
    solana_transaction::Transaction,
    std::{path::PathBuf, str::FromStr},
};

static PROGRAM_ID: Pubkey = crate::ID;

pub fn is_account_closed(account: &Account) -> bool {
    use anchor_lang::system_program;

    account.lamports == 0 && account.data.is_empty() && account.owner == system_program::ID
}
pub struct EscrowTestBuilder {
    program: LiteSVM,
    maker: Keypair,
    taker: Option<Keypair>,
    mint_a: Option<Pubkey>,
    mint_b: Option<Pubkey>,
    maker_ata_a: Option<Pubkey>,
    maker_ata_b: Option<Pubkey>,
    taker_ata_a: Option<Pubkey>,
    taker_ata_b: Option<Pubkey>,
    escrow: Option<Pubkey>,
    vault: Option<Pubkey>,
    last_tx: Option<TransactionMetadata>,
    last_tx_error: Option<String>,
}

impl EscrowTestBuilder {
    pub fn new() -> Self {
        let mut program = LiteSVM::new();
        let maker = Keypair::new();

        program
            .airdrop(&maker.pubkey(), 30 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to maker");

        let so_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/anchor_escrow.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        program.add_program(PROGRAM_ID, &program_data);

        Self {
            program,
            maker,
            taker: None,
            mint_a: None,
            mint_b: None,
            maker_ata_a: None,
            maker_ata_b: None,
            taker_ata_a: None,
            taker_ata_b: None,
            escrow: None,
            vault: None,
            last_tx: None,
            last_tx_error: None,
        }
    }

    pub fn with_devnet_account(mut self, address: &str) -> Self {
        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address = Address::from_str(address).unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        self.program
            .set_account(
                self.maker.pubkey(),
                Account {
                    lamports: fetched_account.lamports,
                    data: fetched_account.data,
                    owner: Pubkey::from(fetched_account.owner.to_bytes()),
                    executable: fetched_account.executable,
                    rent_epoch: fetched_account.rent_epoch,
                },
            )
            .unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);
        self
    }

    pub fn create_mints(mut self) -> Self {
        let mint_a = CreateMint::new(&mut self.program, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut self.program, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();

        self.mint_a = Some(mint_a);
        self.mint_b = Some(mint_b);
        self
    }

    pub fn create_maker_ata_a(mut self) -> Self {
        let mint_a = self.mint_a.expect("Mint A not created");
        let maker_ata_a =
            CreateAssociatedTokenAccount::new(&mut self.program, &self.maker, &mint_a)
                .owner(&self.maker.pubkey())
                .send()
                .unwrap();

        self.maker_ata_a = Some(maker_ata_a);
        self
    }

    pub fn create_maker_ata_b(mut self) -> Self {
        let mint_b = self.mint_b.expect("Mint B not created");
        let maker_ata_b =
            CreateAssociatedTokenAccount::new(&mut self.program, &self.maker, &mint_b)
                .owner(&self.maker.pubkey())
                .send()
                .unwrap();

        self.maker_ata_b = Some(maker_ata_b);
        self
    }

    pub fn mint_to_maker_ata_a(mut self, amount: u64) -> Self {
        MintTo::new(
            &mut self.program,
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
            &mut self.program,
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
        self.program
            .airdrop(&taker.pubkey(), 20 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to taker");

        self.taker = Some(taker);
        self
    }

    pub fn create_taker_atas(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");
        let mint_a = self.mint_a.expect("Mint A not created");
        let mint_b = self.mint_b.expect("Mint B not created");

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut self.program, taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut self.program, taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        self.taker_ata_a = Some(taker_ata_a);
        self.taker_ata_b = Some(taker_ata_b);
        self
    }

    pub fn advance_time(mut self, seconds: i64) -> Self {
        use anchor_lang::prelude::Clock;
        let mut clock = self.program.get_sysvar::<Clock>();
        clock.unix_timestamp += seconds;
        self.program.set_sysvar::<Clock>(&clock);
        self
    }

    pub fn execute_make(mut self, deposit: u64, seed: u64, receive: u64) -> Self {
        let escrow = Pubkey::find_program_address(
            &[b"escrow", self.maker.pubkey().as_ref(), &seed.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;

        let vault = associated_token::get_associated_token_address(
            &escrow,
            &self.mint_a.expect("Mint A not created"),
        );

        self.escrow = Some(escrow);
        self.vault = Some(vault);

        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                mint_b: self.mint_b.unwrap(),
                maker_ata_a: self.maker_ata_a.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit,
                seed,
                receive,
            }
            .data(),
        };

        let message = Message::new(&[make_ix], Some(&self.maker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&self.maker], message, recent_blockhash);
        let tx = self.program.send_transaction(transaction).unwrap();

        msg!("\n\nMake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;
        self
    }

    pub fn execute_take(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                mint_b: self.mint_b.unwrap(),
                taker_ata_a: self.taker_ata_a.unwrap(),
                taker_ata_b: self.taker_ata_b.unwrap(),
                maker_ata_b: self.maker_ata_b.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[taker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction);

        match &tx {
            Ok(tx_result) => {
                msg!("\n\nTake transaction successful");
                msg!("CUs Consumed: {}", tx_result.compute_units_consumed);
                msg!("Tx Signature: {}", tx_result.signature);
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

    pub fn execute_refund(mut self) -> Self {
        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                maker_ata_a: self.maker_ata_a.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&self.maker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&self.maker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction).unwrap();

        msg!("\n\nRefund transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;
        self
    }

    pub fn get_vault_data(&self) -> spl_token::state::Account {
        let vault_account = self.program.get_account(&self.vault.unwrap()).unwrap();
        spl_token::state::Account::unpack(&vault_account.data).unwrap()
    }

    pub fn get_escrow_data(&self) -> crate::state::Escrow {
        let escrow_account = self.program.get_account(&self.escrow.unwrap()).unwrap();
        crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap()
    }

    pub fn get_maker_ata_a_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.maker_ata_a.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_maker_ata_b_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.maker_ata_b.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_taker_ata_a_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.taker_ata_a.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_taker_ata_b_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.taker_ata_b.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn is_vault_closed(&self) -> bool {
        let account = self.program.get_account(&self.vault.unwrap()).unwrap();
        is_account_closed(&account)
    }

    pub fn is_escrow_closed(&self) -> bool {
        let account = self.program.get_account(&self.escrow.unwrap()).unwrap();
        is_account_closed(&account)
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

    pub fn escrow(&self) -> Pubkey {
        self.escrow.unwrap()
    }

    pub fn last_tx_succeeded(&self) -> bool {
        self.last_tx_error.is_none()
    }

    pub fn last_tx_failed(&self) -> bool {
        self.last_tx_error.is_some()
    }
}
