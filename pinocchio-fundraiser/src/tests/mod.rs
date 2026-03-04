mod fundraiser_test_builder;

#[cfg(test)]
mod tests {
    use solana_keypair::Keypair;
    use solana_signer::Signer;

    use crate::tests::fundraiser_test_builder::FundraiserTestBuilder;

    #[test]
    fn test_initialize() {
        let amount_to_raise: u64 = 30000000;
        let duration: u8 = 0;

        let builder = FundraiserTestBuilder::new()
            .create_mint()
            .execute_initialize(amount_to_raise, duration);

        let vault_data = builder.vault_data();
        assert_eq!(vault_data.amount(), 0);
        assert_eq!(vault_data.owner(), &builder.fundraiser_pubkey());
        assert_eq!(vault_data.mint(), &builder.mint());

        let fundraiser_data = builder.fundraiser_data();
        assert_eq!(fundraiser_data.duration, duration);
        assert_eq!(fundraiser_data.maker, *builder.maker_pubkey().as_ref());
        assert_eq!(fundraiser_data.mint_to_raise, *builder.mint().as_ref());
        assert_eq!(fundraiser_data.amount_to_raise, amount_to_raise.to_le_bytes());
    }

    #[test]
    fn test_contribute() {
        let amount_to_raise: u64 = 30000000;
        let duration: u8 = 0;

        let contributor1 = Keypair::new();
        let amount: u64 = 1000000;

        let builder = FundraiserTestBuilder::new()
            .create_mint()
            .execute_initialize(amount_to_raise, duration)
            .setup_contributor(&contributor1, amount)
            .execute_contribute(&contributor1, amount);

        assert!(builder.last_tx_succeeded());

        let vault_data = builder.vault_data();
        assert_eq!(vault_data.amount(), amount);

        let fundraiser_data = builder.fundraiser_data();
        assert_eq!(fundraiser_data.current_amount(), amount);

        let contributor_ata_data = builder.contributor_ata_data(&contributor1.pubkey());
        assert_eq!(contributor_ata_data.amount(), 0);
    }

    #[test]
    fn test_refund() {
        let amount_to_raise: u64 = 30000000;
        let duration: u8 = 0;

        let contributor1 = Keypair::new();
        let amount: u64 = 1000000;

        let builder = FundraiserTestBuilder::new()
            .create_mint()
            .execute_initialize(amount_to_raise, duration)
            .setup_contributor(&contributor1, amount)
            .execute_contribute(&contributor1, amount)
            .execute_refund(&contributor1);

        assert!(builder.last_tx_succeeded());

        let vault_data = builder.vault_data();
        assert_eq!(vault_data.amount(), 0);

        let fundraiser_data = builder.fundraiser_data();
        assert_eq!(fundraiser_data.current_amount(), 0);
 
        let contributor_ata_data = builder.contributor_ata_data(&contributor1.pubkey());
        assert_eq!(contributor_ata_data.amount(), amount);

        assert!(builder.is_contributor_closed(&contributor1.pubkey()), "Contributor account should be closed");
    }

    #[test]
    fn test_checker() {
        let amount_to_raise: u64 = 10_000_000;
        let duration: u8 = 0;

        let contributor1 = Keypair::new();
        let contributor2 = Keypair::new();
        let contributor3 = Keypair::new();
        let contributor4 = Keypair::new();
        let contributor5 = Keypair::new();
        let contributor6 = Keypair::new();
        let contributor7 = Keypair::new();
        let contributor8 = Keypair::new();
        let contributor9 = Keypair::new();
        let contributor10: Keypair = Keypair::new();
        let mint_amount: u64 = 4_000_000;
        let amount: u64 = 1_000_000;

        let builder = FundraiserTestBuilder::new()
            .create_mint()
            .create_maker_ata()
            .execute_initialize(amount_to_raise, duration)
            .setup_contributor(&contributor1, mint_amount)
            .setup_contributor(&contributor2, mint_amount)
            .setup_contributor(&contributor3, mint_amount)
            .setup_contributor(&contributor4, mint_amount)
            .setup_contributor(&contributor5, mint_amount)
            .setup_contributor(&contributor6, mint_amount)
            .setup_contributor(&contributor7, mint_amount)
            .setup_contributor(&contributor8, mint_amount)
            .setup_contributor(&contributor9, mint_amount)
            .setup_contributor(&contributor10, mint_amount)
            .execute_contribute(&contributor1, amount)
            .execute_contribute(&contributor2, amount)
            .execute_contribute(&contributor3, amount)
            .execute_contribute(&contributor4, amount)
            .execute_contribute(&contributor5, amount)
            .execute_contribute(&contributor6, amount)
            .execute_contribute(&contributor7, amount)
            .execute_contribute(&contributor8, amount)
            .execute_contribute(&contributor9, amount)
            .execute_contribute(&contributor10, amount)
            .execute_checker();

        assert!(builder.last_tx_succeeded());

        let maker_ata_data = builder.maker_ata_data();
        assert_eq!(maker_ata_data.amount(), amount_to_raise);

        assert!(builder.is_vault_ata_closed(), "Vault should be closed");
        assert!(builder.is_fundraiser_closed(), "Fundraiser account should be closed");
    }

}