#![cfg(test)]

use {
    crate::state::Vault,
    anchor_lang::{
        AccountDeserialize, InstructionData, prelude::{Pubkey, ToAccountMetas}
    },
    anchor_spl::associated_token::spl_associated_token_account,
    litesvm::{
        LiteSVM, types::{FailedTransactionMetadata, TransactionMetadata}
    },
    solana_address::Address,
    solana_message::Message,
    solana_sdk::{
        instruction::Instruction, signature::Keypair, signer::Signer, transaction::Transaction,
    },
    spl_token_2022::{
        self,
        extension::{
            BaseStateWithExtensions, StateWithExtensions, mint_close_authority::MintCloseAuthority
        },
        state::Mint,
    },
};

const PROGRAM_ID: Pubkey = crate::ID;
const HOOK_PROGRAM_ID: Pubkey = whitelist_transfer_hook::ID;

pub fn to_address(p: Pubkey) -> Address {
    Address::new_from_array(p.to_bytes())
}

pub fn to_pubkey(a: Address) -> Pubkey {
    Pubkey::new_from_array(a.to_bytes())
}

fn map_ix(
    ix: anchor_lang::solana_program::instruction::Instruction,
) -> solana_sdk::instruction::Instruction {
    let accounts: Vec<_> = ix
        .accounts
        .into_iter()
        .map(|m| solana_sdk::instruction::AccountMeta {
            pubkey: to_address(m.pubkey),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect();

    solana_sdk::instruction::Instruction {
        program_id: to_address(ix.program_id),
        accounts,
        data: ix.data,
    }
}

fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[solana_sdk::instruction::Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let message = Message::new(ixs, Some(&payer.pubkey()));
    let recent_blockhash = svm.latest_blockhash();
    let transaction: Transaction = Transaction::new(signers, message, recent_blockhash);
    let tx = svm.send_transaction(transaction);

    tx
}

pub struct WhitelistVaultTestBuilder {
    svm: LiteSVM,
    admin: Keypair,
    mint: Pubkey,
    vault: Pubkey,
    vault_ata: Pubkey,
    extra_meta: Pubkey,
    last_tx: Option<TransactionMetadata>,
    last_tx_error: Option<String>,
    config: Pubkey,
}

impl WhitelistVaultTestBuilder {
    pub fn new() -> Self {
        let admin = Keypair::new();
        let admin_address = admin.pubkey();

        let mint = WhitelistVaultTestBuilder::get_mint_pda();

        let mut svm = LiteSVM::new();

        let whitelist_vault_bytes = std::fs::read("../../target/deploy/whitelist_vault.so")
            .expect("Run anchor build first");

        svm.add_program(to_address(PROGRAM_ID), &whitelist_vault_bytes)
            .expect("Failed to add program whitelist_vault");

        let whitelist_transfer_hook_bytes =
            std::fs::read("../../target/deploy/whitelist_transfer_hook.so")
                .expect("Run anchor build first");

        svm.add_program(to_address(HOOK_PROGRAM_ID), &whitelist_transfer_hook_bytes)
            .expect("Failed to add program");

        svm.airdrop(&admin_address, 10_000_000_000).unwrap();

        let (vault, _) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);

        let vault_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &vault,
            &mint,
            &spl_token_2022::ID,
        );

        let (config, _) = Pubkey::find_program_address(&[b"config"], &HOOK_PROGRAM_ID);

        let (extra_meta, _) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint.as_ref()],
            &HOOK_PROGRAM_ID,
        );

        Self {
            svm,
            admin,
            mint,
            vault,
            vault_ata,
            config,
            extra_meta,
            last_tx: None,
            last_tx_error: None,
        }
    }

    pub fn initialize(mut self) -> Self {
        let initialize_vault_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                admin: to_pubkey(self.admin.pubkey()),
                vault: self.vault,
                mint: self.mint,
                vault_ata: self.vault_ata,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: anchor_lang::system_program::ID,
                token_program: spl_token_2022::ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        });

        let initialize_transfer_hook_ix =
            map_ix(anchor_lang::solana_program::instruction::Instruction {
                program_id: HOOK_PROGRAM_ID,
                accounts: whitelist_transfer_hook::accounts::Initialize {
                    admin: to_pubkey(self.admin.pubkey()),
                    mint: self.mint,
                    system_program: anchor_lang::system_program::ID,
                    config: self.config,
                    extra_account_meta_list: self.extra_meta,
                }
                .to_account_metas(None),
                data: whitelist_transfer_hook::instruction::Initialize {}.data(),
            });

        let tx_result = send_tx(
            &mut self.svm,
            &[initialize_vault_ix, initialize_transfer_hook_ix],
            &self.admin,
            &[&self.admin],
        );

        self.set_tx_result(tx_result)
    }

    pub fn create_user_ata(mut self, user: &Keypair) -> Self {
        self.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

        let user_pubkey = to_pubkey(user.pubkey());

        let create_associated_token_account_ix = map_ix(
            spl_associated_token_account::instruction::create_associated_token_account(
                &user_pubkey,
                &user_pubkey,
                &self.mint,
                &spl_token_2022::ID,
            ),
        );

        let tx_result = send_tx(
            &mut self.svm,
            &[create_associated_token_account_ix],
            &user,
            &[&user],
        );
        self.set_tx_result(tx_result)
    }

    pub fn mint_tokens_to(mut self, user: &Keypair, amount: u64) -> Self {
        let admin_pubkey = to_pubkey(self.admin.pubkey());

        let user_pubkey = to_pubkey(user.pubkey());

        let user_ata = self.get_user_ata(&user_pubkey);

        let mint_tokens_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::MintTokens {
                admin: admin_pubkey,
                user: user_pubkey,
                user_ata,
                vault: self.vault,
                mint: self.mint,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: anchor_lang::system_program::ID,
                token_program: spl_token_2022::ID,
            }
            .to_account_metas(None),
            data: crate::instruction::MintTokens { amount }.data(),
        });

        let tx_result = send_tx(
            &mut self.svm,
            &[mint_tokens_ix],
            &self.admin,
            &[&self.admin],
        );

        self.set_tx_result(tx_result)
    }

    pub fn deposit(mut self, user: &Keypair, amount: u64) -> Self {
        let user_pubkey = to_pubkey(user.pubkey());
        let user_ata = self.get_user_ata(&user_pubkey);
        let user_deposit = WhitelistVaultTestBuilder::get_user_deposit_pda(&user_pubkey);

        let source_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&user_pubkey);
        let destination_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&self.vault);

        let accounts = crate::accounts::Deposit {
            user: user_pubkey,
            mint: self.mint,
            user_ata,
            vault: self.vault,
            source_whitelist,
            destination_whitelist,
            vault_ata: self.vault_ata,
            extra_account_meta_list: self.extra_meta,
            hook_program: HOOK_PROGRAM_ID,
            associated_token_program: anchor_spl::associated_token::ID,
            token_program: spl_token_2022::ID,
            system_program: anchor_lang::system_program::ID,
            user_deposit,
        }
        .to_account_metas(None);

        let deposit_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: crate::instruction::Deposit { amount }.data(),
        });

        let tx_result = send_tx(&mut self.svm, &[deposit_ix], user, &[&user]);

        self.set_tx_result(tx_result)
    }

    pub fn withdraw(mut self, user: &Keypair, amount: u64) -> Self {
        let user_pubkey = to_pubkey(user.pubkey());
        let user_ata = self.get_user_ata(&user_pubkey);
        let user_deposit = WhitelistVaultTestBuilder::get_user_deposit_pda(&user_pubkey);

        let source_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&self.vault);
        let destination_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&user_pubkey);

        let accounts = crate::accounts::Withdraw {
            user: user_pubkey,
            mint: self.mint,
            user_ata,
            vault: self.vault,
            vault_ata: self.vault_ata,
            source_whitelist,
            destination_whitelist,
            extra_account_meta_list: self.extra_meta,
            hook_program: HOOK_PROGRAM_ID,
            associated_token_program: anchor_spl::associated_token::ID,
            token_program: spl_token_2022::ID,
            system_program: anchor_lang::system_program::ID,
            user_deposit,
        }
        .to_account_metas(None);

        let withdraw_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: crate::instruction::Withdraw { amount }.data(),
        });

        let tx_result = send_tx(&mut self.svm, &[withdraw_ix], user, &[&user]);

        self.set_tx_result(tx_result)
    }

    pub fn add_user_to_whitelist(mut self, user: &Keypair) -> Self {
        let user_pubkey = to_pubkey(user.pubkey());

        let whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&user_pubkey);

        let accounts = whitelist_transfer_hook::accounts::AddToWhitelist {
            admin: to_pubkey(self.admin.pubkey()),
            whitelist,
            config: self.config,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None);

        let add_whitelist_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts,
            data: whitelist_transfer_hook::instruction::AddToWhitelist { user: user_pubkey }.data(),
        });

        let tx_result = send_tx(
            &mut self.svm,
            &[add_whitelist_ix],
            &self.admin,
            &[&self.admin],
        );

        self.set_tx_result(tx_result)
    }

    pub fn remove_user_from_whitelist(mut self, user: &Keypair) -> Self {
        let user_pubkey = to_pubkey(user.pubkey());

        let whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&user_pubkey);

        let accounts = whitelist_transfer_hook::accounts::RemoveFromWhitelist {
            admin: to_pubkey(self.admin.pubkey()),
            whitelist,
            config: self.config,
        }
        .to_account_metas(None);

        let remove_whitelist_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts,
            data: whitelist_transfer_hook::instruction::RemoveFromWhitelist { user: user_pubkey }.data(),
        });

        let tx_result = send_tx(
            &mut self.svm,
            &[remove_whitelist_ix],
            &self.admin,
            &[&self.admin],
        );

        self.set_tx_result(tx_result)
    }

    pub fn transfer_tokens(mut self, from_user: &Keypair, to_user: &Pubkey, amount: u64) -> Self {
        let from_pubkey = to_pubkey(from_user.pubkey());
        let from_ata = self.get_user_ata(&from_pubkey);

        let to_pubkey = to_user;
        let to_ata = self.get_user_ata(to_pubkey);

        let source_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&from_pubkey);
        let destination_whitelist = WhitelistVaultTestBuilder::get_whitelist_pda(&to_pubkey);

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(to_address(from_ata), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(self.mint), false),
            solana_sdk::instruction::AccountMeta::new(to_address(to_ata), false),
            solana_sdk::instruction::AccountMeta::new(to_address(from_pubkey), true),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(self.extra_meta), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(source_whitelist), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(destination_whitelist), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(HOOK_PROGRAM_ID), false),
        ];

        let transfer_ix = Instruction {
            program_id: to_address(spl_token_2022::ID),
            accounts,
            data: spl_token_2022::instruction::TokenInstruction::TransferChecked {
                amount,
                decimals: 9,
            }
            .pack(),
        };

        let tx_result = send_tx(&mut self.svm, &[transfer_ix], &from_user, &[&from_user]);

        self.set_tx_result(tx_result)
    }

    pub fn get_mint_pda() -> Pubkey {
        let (mint, _) = Pubkey::find_program_address(&[b"mint"], &PROGRAM_ID);

        mint
    }

    pub fn vault(&self) -> Pubkey {
        self.vault
    }

    pub fn admin(&self) -> Pubkey {
        to_pubkey(self.admin.pubkey())
    }

    pub fn get_whitelist_pda(user_pubkey: &Pubkey) -> Pubkey {
        let (whitelist, _) =
            Pubkey::find_program_address(&[b"whitelist", user_pubkey.as_ref()], &HOOK_PROGRAM_ID);

        whitelist
    }

    pub fn get_user_deposit_pda(user_pubkey: &Pubkey) -> Pubkey {
        let (user_deposit, _) =
            Pubkey::find_program_address(&[b"user_deposit", user_pubkey.as_ref()], &PROGRAM_ID);

        user_deposit
    }

    pub fn get_vault_data(&self) -> Vault {
        let vault_account = self.svm.get_account(&to_address(self.vault)).unwrap();
        Vault::try_deserialize(&mut vault_account.data.as_ref()).unwrap()
    }

    pub fn get_mint_close_authority_extension(&self) -> MintCloseAuthority {
        let mint_account = self
            .svm
            .get_account(&to_address(WhitelistVaultTestBuilder::get_mint_pda()))
            .unwrap();

        let mint_data: &[u8] = mint_account.data.as_slice();

        let mint_state = StateWithExtensions::<Mint>::unpack(mint_data).unwrap();

        *mint_state
            .get_extension::<MintCloseAuthority>()
            .expect("CloseAuthority extension must exist")
    }

    pub fn get_user_ata(&self, user: &Pubkey) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &user,
            &self.mint,
            &spl_token_2022::ID,
        )
    }

    pub fn get_ata_data(&self, user: &Pubkey) -> spl_token_2022::state::Account {
        let user_ata = self.get_user_ata(user);

        let ata_account = self.svm.get_account(&to_address(user_ata)).unwrap();

        let account =
            StateWithExtensions::<spl_token_2022::state::Account>::unpack(&ata_account.data)
                .unwrap();

        account.base
    }

    pub fn last_tx_failed(&self) -> bool {
        self.last_tx_error.is_some()
    }

    fn set_tx_result(mut self, tx: Result<TransactionMetadata, FailedTransactionMetadata>) -> Self {
        match &tx {
            Ok(tx_result) => {
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
}
