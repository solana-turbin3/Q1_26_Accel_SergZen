#[cfg(test)]
mod tests {
    use {
        anchor_lang::{
            AccountDeserialize,
            InstructionData,
            ToAccountMetas,
            prelude::Pubkey as AnchorPubkey,
            solana_program::{
                self,
                instruction::Instruction as AnchorInstruction,
            },
        },
        anchor_spl::associated_token,
        litesvm::LiteSVM,
        litesvm_token::{
            CreateAssociatedTokenAccount,
            CreateMint,
            MintTo,
        },
        solana_sdk::{
            instruction::{Instruction, AccountMeta},
            message::Message,
            native_token::LAMPORTS_PER_SOL,
            pubkey::Pubkey,
            signature::{Keypair, Signer},

            transaction::Transaction,
        },
    };

    use spl_token_2022::state::Account as TokenAccount2022;

    static PROGRAM_ID: AnchorPubkey = crate::ID;

    // --------------------------------------------------
    // Helpers
    // --------------------------------------------------

    fn to_sdk_pubkey(p: AnchorPubkey) -> Pubkey {
        Pubkey::new_from_array(p.to_bytes())
    }

    fn to_sdk_instruction(ix: AnchorInstruction) -> Instruction {
        Instruction {
            program_id: to_sdk_pubkey(ix.program_id),
            accounts: ix.accounts.into_iter().map(|a| AccountMeta {
                pubkey: to_sdk_pubkey(a.pubkey),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }).collect(),
            data: ix.data,
        }
    }

    fn to_anchor_pubkey(p: Pubkey) -> AnchorPubkey {
        AnchorPubkey::new_from_array(p.to_bytes())
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        let program_bytes = std::fs::read("../../target/deploy/whitelist_vault.so")
            .expect("Failed to read program file. Run anchor build first.");

        svm.add_program(
            to_sdk_pubkey(PROGRAM_ID),
            &program_bytes,
        );

        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        (svm, payer)
    }

    fn vault_pda() -> (AnchorPubkey, u8) {
        AnchorPubkey::find_program_address(&[b"vault"], &PROGRAM_ID)
    }

    fn whitelist_pda() -> (AnchorPubkey, u8) {
        AnchorPubkey::find_program_address(&[b"whitelist"], &PROGRAM_ID)
    }

    // --------------------------------------------------
    // Initialize
    // --------------------------------------------------

    #[test]
    fn test_initialize() {
        let (mut svm, admin) = setup();

        let mint = CreateMint::new(&mut svm, &admin)
            .decimals(9)
            .authority(&admin.pubkey())
            .send()
            .unwrap();
        let mint_anchor = to_anchor_pubkey(mint);
        let admin_anchor = to_anchor_pubkey(admin.pubkey());

        let (vault, _) = vault_pda();
        let (whitelist, _) = whitelist_pda();

        let vault_ata =
            associated_token::get_associated_token_address(&vault, &mint_anchor);

        let extra_meta =
            AnchorPubkey::find_program_address(
                &[b"extra-account-metas", mint_anchor.as_ref()],
                &PROGRAM_ID,
            )
            .0;

        let anchor_ix = AnchorInstruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                admin: admin_anchor,
                vault,
                whitelist,
                mint: mint_anchor,
                vault_ata,
                associated_token_program: associated_token::ID,
                system_program: solana_program::system_program::ID,
                token_program: spl_token_2022::ID,
                extra_account_meta_list: extra_meta,
            }
            .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };
        let ix = to_sdk_instruction(anchor_ix);

        svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        ))
        .unwrap();
    }

    // --------------------------------------------------
    // Deposit
    // --------------------------------------------------

    #[test]
    fn test_deposit_whitelists_user() {
        let (mut svm, admin) = setup();

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 2 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &admin)
            .decimals(9)
            .authority(&admin.pubkey())
            .send()
            .unwrap();
        let mint_anchor = to_anchor_pubkey(mint);
        let user_anchor = to_anchor_pubkey(user.pubkey());

        let user_ata = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
            .owner(&user.pubkey())
            .send()
            .unwrap();
        let user_ata_anchor = to_anchor_pubkey(user_ata);

        MintTo::new(&mut svm, &admin, &mint, &user_ata, 1_000)
            .send()
            .unwrap();

        test_initialize();

        let (vault, _) = vault_pda();
        let (whitelist, _) = whitelist_pda();

        let vault_ata =
            associated_token::get_associated_token_address(&vault, &mint_anchor);

        let anchor_ix = AnchorInstruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: user_anchor,
                mint: mint_anchor,
                user_ata: user_ata_anchor,
                vault,
                whitelist,
                vault_ata,
                associated_token_program: associated_token::ID,
                token_program: spl_token_2022::ID,
                system_program: solana_program::system_program::ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Deposit { amount: 500 }.data(),
        };
        let ix = to_sdk_instruction(anchor_ix);

        svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix],
            Some(&user.pubkey()),
            &[&user],
            svm.latest_blockhash(),
        ))
        .unwrap();

        let whitelist_acc = svm.get_account(&to_sdk_pubkey(whitelist)).unwrap();
        let wl =
            crate::state::Whitelist::try_deserialize(
                &mut whitelist_acc.data.as_ref(),
            )
            .unwrap();

        assert_eq!(wl.address.len(), 1);
        assert_eq!(wl.amount[0], 500);
    }

    // --------------------------------------------------
    // Withdraw
    // --------------------------------------------------

    #[test]
    fn test_withdraw_fails_if_overdrawn() {
        let (mut svm, admin) = setup();
        let user = Keypair::new();

        svm.airdrop(&user.pubkey(), 2 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &admin)
            .decimals(9)
            .authority(&admin.pubkey())
            .send()
            .unwrap();
        let mint_anchor = to_anchor_pubkey(mint);
        let user_anchor = to_anchor_pubkey(user.pubkey());

        let user_ata = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
            .owner(&user.pubkey())
            .send()
            .unwrap();
        let user_ata_anchor = to_anchor_pubkey(user_ata);

        test_initialize();

        let (vault, _) = vault_pda();
        let (whitelist, _) = whitelist_pda();

        let vault_ata =
            associated_token::get_associated_token_address(&vault, &mint_anchor);

        let anchor_ix = AnchorInstruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Withdraw {
                user: user_anchor,
                mint: mint_anchor,
                user_ata: user_ata_anchor,
                vault,
                whitelist,
                vault_ata,
                associated_token_program: associated_token::ID,
                token_program: spl_token_2022::ID,
                system_program: solana_program::system_program::ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Withdraw { amount: 1 }.data(),
        };
        let ix = to_sdk_instruction(anchor_ix);

        let err = svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix],
            Some(&user.pubkey()),
            &[&user],
            svm.latest_blockhash(),
        ));

        assert!(err.is_err());
    }

    // --------------------------------------------------
    // Transfer Hook
    // --------------------------------------------------

    #[test]
    fn test_transfer_hook_blocks_non_whitelisted() {
        let (mut svm, admin) = setup();

        let alice = Keypair::new();
        let bob = Keypair::new();

        svm.airdrop(&alice.pubkey(), 2 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &admin)
            .decimals(9)
            .authority(&admin.pubkey())
            .send()
            .unwrap();
        let mint_anchor = to_anchor_pubkey(mint);

        let alice_ata =
            CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
                .owner(&alice.pubkey())
                .send()
                .unwrap();
        let alice_ata_anchor = to_anchor_pubkey(alice_ata);

        let bob_ata =
            CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
                .owner(&bob.pubkey())
                .send()
                .unwrap();
        let bob_ata_anchor = to_anchor_pubkey(bob_ata);

        MintTo::new(&mut svm, &admin, &mint, &alice_ata, 100)
            .send()
            .unwrap();

        // Alice is NOT whitelisted → transfer should fail
        let ix = to_sdk_instruction(spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::ID,
            &alice_ata_anchor,
            &mint_anchor,
            &bob_ata_anchor,
            &to_anchor_pubkey(alice.pubkey()),
            &[],
            10,
            9,
        )
        .unwrap());

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&alice.pubkey()),
            &[&alice],
            svm.latest_blockhash(),
        );

        assert!(svm.send_transaction(tx).is_err());
    }
}
