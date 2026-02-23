mod whitelist_vault_test_builder;

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::Pubkey;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use crate::tests::whitelist_vault_test_builder::{WhitelistVaultTestBuilder, to_pubkey};

    #[test]
    fn initialize() {
        let builder = WhitelistVaultTestBuilder::new()
            .initialize();

        let vault_data = builder.get_vault_data();

        assert_eq!(vault_data.balance, 0);

        let mint_close_authority_ext = builder.get_mint_close_authority_extension();
        let authority: Option<Pubkey> = mint_close_authority_ext.close_authority.into();
        let admin = builder.admin();

        assert_eq!(authority, Some(admin));
    }

    #[test]
    fn user_to_user_transfer_allowed_for_whitelisted() {
        let user1 = Keypair::new();
        let user2 = Keypair::new();

        let amount = 400_000;

        let builder = WhitelistVaultTestBuilder::new()
            .initialize()
            .create_user_ata(&user1)
            .create_user_ata(&user2)
            .mint_tokens_to(&user1, amount)
            .add_user_to_whitelist(&user1)
            .transfer_tokens(&user1, &to_pubkey(user2.pubkey()), amount);

        let user1_ata_data = builder.get_ata_data(&to_pubkey(user1.pubkey()));
        let user2_ata_data = builder.get_ata_data(&to_pubkey(user2.pubkey()));

        assert_eq!(user1_ata_data.amount, 0);
        assert_eq!(user1_ata_data.owner, to_pubkey(user1.pubkey()));
        assert_eq!(user1_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());

        assert_eq!(user2_ata_data.amount, amount);
        assert_eq!(user2_ata_data.owner, to_pubkey(user2.pubkey()));
        assert_eq!(user2_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());
    }

    #[test]
    fn test_deposit_success() {
        let user1 = Keypair::new();

        let amount = 500_000;

        let builder = WhitelistVaultTestBuilder::new()
            .initialize()
            .create_user_ata(&user1)
            .add_user_to_whitelist(&user1)
            .mint_tokens_to(&user1, amount)
            .deposit(&user1, amount);

        let vault = builder.vault();

        let user1_ata_data = builder.get_ata_data(&to_pubkey(user1.pubkey()));
        let vault_ata_data = builder.get_ata_data(&vault);

        assert_eq!(user1_ata_data.amount, 0);
        assert_eq!(user1_ata_data.owner, to_pubkey(user1.pubkey()));
        assert_eq!(user1_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());

        assert_eq!(vault_ata_data.amount, amount);
        assert_eq!(vault_ata_data.owner, vault);
        assert_eq!(vault_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());

        let vault_data = builder.get_vault_data();

        assert_eq!(vault_data.balance, amount);
    }

    #[test]
    fn test_withdraw_success() {
        let user1 = Keypair::new();

        let amount = 500_000;
        let withdraw_amount = 100_000;

        let builder = WhitelistVaultTestBuilder::new()
            .initialize()
            .create_user_ata(&user1)
            .add_user_to_whitelist(&user1)
            .mint_tokens_to(&user1, amount)
            .deposit(&user1, amount)
            .withdraw(&user1, withdraw_amount);

        let vault = builder.vault();

        let user1_ata_data = builder.get_ata_data(&to_pubkey(user1.pubkey()));
        let vault_ata_data = builder.get_ata_data(&vault);

        assert_eq!(user1_ata_data.amount, withdraw_amount);
        assert_eq!(user1_ata_data.owner, to_pubkey(user1.pubkey()));
        assert_eq!(user1_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());

        assert_eq!(vault_ata_data.amount, amount - withdraw_amount);
        assert_eq!(vault_ata_data.owner, vault);
        assert_eq!(vault_ata_data.mint, WhitelistVaultTestBuilder::get_mint_pda());

        let vault_data = builder.get_vault_data();

        assert_eq!(vault_data.balance, amount - withdraw_amount);
    }

    #[test]
    fn test_not_whitelisted_user_cannot_deposit() {
        let user1 = Keypair::new();

        let amount = 500_000;

        let builder = WhitelistVaultTestBuilder::new()
            .initialize()
            .create_user_ata(&user1)
            .mint_tokens_to(&user1, amount)
            .deposit(&user1, amount);

        assert!(builder.last_tx_failed(), "Deposit should fail without whitelist");
    }

    #[test]
    fn test_not_whitelisted_user_cannot_withdraw() {
        let user1 = Keypair::new();

        let amount = 500_000;
        let withdraw_amount = 100_000;

        let builder = WhitelistVaultTestBuilder::new()
            .initialize()
            .create_user_ata(&user1)
            .add_user_to_whitelist(&user1)
            .mint_tokens_to(&user1, amount)
            .deposit(&user1, amount)
            .remove_user_from_whitelist(&user1)
            .withdraw(&user1, withdraw_amount);

        assert!(builder.last_tx_failed(), "Withdraw should fail without whitelist");
    }
}