mod escrow_test_builder;

#[cfg(test)]
mod tests {
    use crate::tests::escrow_test_builder::EscrowTestBuilder;

    #[test]
    fn test_make() {
        let amount_to_receive: u64 = 100000000; // 100 tokens with 6 decimal places
        let amount_to_give: u64 = 500000000; // 500 tokens with 6 decimal places

        let seed = 123u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(amount_to_give)
            .set_escrow_accounts(seed)
            .execute_make(amount_to_give, seed, amount_to_receive);

        let escrow_ata_data = builder.escrow_ata_data();
        assert_eq!(escrow_ata_data.amount(), amount_to_give);
        assert_eq!(escrow_ata_data.owner(), &builder.escrow_pubkey());
        assert_eq!(escrow_ata_data.mint(), &builder.mint_a());

        let escrow_data = builder.escrow_data();
        assert_eq!(escrow_data.seed(), seed);
        assert_eq!(escrow_data.maker(), builder.maker_pubkey());
        assert_eq!(escrow_data.mint_a(), builder.mint_a());
        assert_eq!(escrow_data.mint_b(), builder.mint_b());
        assert_eq!(escrow_data.amount_to_receive(), amount_to_receive);
        assert_eq!(escrow_data.amount_to_give(), amount_to_give);
    }

    #[test]
    fn test_take() {
        let deposit = 20u64;
        let seed = 123u64;
        let receive = 30u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(deposit)
            .set_escrow_accounts(seed)
            .execute_make(deposit, seed, receive)
            .setup_taker()
            .create_maker_ata_b()
            .create_taker_atas()
            .mint_to_taker_ata_b(receive)
            .execute_take();

        assert!(builder.last_tx_succeeded());

        let taker_ata_a_data = builder.taker_ata_a_data();
        assert_eq!(taker_ata_a_data.amount(), deposit);

        let taker_ata_b_data = builder.taker_ata_b_data();
        assert_eq!(taker_ata_b_data.amount(), 0);

        let maker_ata_b_data = builder.maker_ata_b_data();
        assert_eq!(maker_ata_b_data.amount(), receive);

        assert!(builder.is_escrow_ata_closed(), "Escrow ATA should be closed");

        assert!(builder.is_escrow_closed(), "Escrow should be closed");
    }

    #[test]
    fn test_cancel() {
        let deposit = 20u64;
        let seed = 123u64;
        let receive = 30u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(deposit)
            .set_escrow_accounts(seed)
            .execute_make(deposit, seed, receive)
            .execute_cancel();

        let maker_ata_a_data = builder.maker_ata_a_data();
        assert_eq!(maker_ata_a_data.amount(), deposit);
        assert_eq!(*maker_ata_a_data.owner(), builder.maker_pubkey());
        assert_eq!(*maker_ata_a_data.mint(), builder.mint_a());

        assert!(builder.is_escrow_ata_closed(), "Escrow ATA should be closed"); 

        assert!(builder.is_escrow_closed(), "Escrow should be closed");
    }

    #[test]
    fn test_make_v2() {
        let amount_to_receive: u64 = 100000000; // 100 tokens with 6 decimal places
        let amount_to_give: u64 = 500000000; // 500 tokens with 6 decimal places

        let seed = 123u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(amount_to_give)
            .set_escrow_accounts(seed)
            .execute_make_v2(amount_to_give, seed, amount_to_receive);

        let escrow_ata_data = builder.escrow_ata_data();
        assert_eq!(escrow_ata_data.amount(), amount_to_give);
        assert_eq!(escrow_ata_data.owner(), &builder.escrow_pubkey());
        assert_eq!(escrow_ata_data.mint(), &builder.mint_a());

        let escrow_data = builder.escrow_data();
        assert_eq!(escrow_data.seed(), seed);
        assert_eq!(escrow_data.maker(), builder.maker_pubkey());
        assert_eq!(escrow_data.mint_a(), builder.mint_a());
        assert_eq!(escrow_data.mint_b(), builder.mint_b());
        assert_eq!(escrow_data.amount_to_receive(), amount_to_receive);
        assert_eq!(escrow_data.amount_to_give(), amount_to_give);
    }

    #[test]
    fn test_take_v2() {
        let deposit = 20u64;
        let seed = 123u64;
        let receive = 30u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(deposit)
            .set_escrow_accounts(seed)
            .execute_make_v2(deposit, seed, receive)
            .setup_taker()
            .create_maker_ata_b()
            .create_taker_atas()
            .mint_to_taker_ata_b(receive)
            .execute_take_v2();

        assert!(builder.last_tx_succeeded());

        let taker_ata_a_data = builder.taker_ata_a_data();
        assert_eq!(taker_ata_a_data.amount(), deposit);

        let taker_ata_b_data = builder.taker_ata_b_data();
        assert_eq!(taker_ata_b_data.amount(), 0);

        let maker_ata_b_data = builder.maker_ata_b_data();
        assert_eq!(maker_ata_b_data.amount(), receive);

        assert!(builder.is_escrow_ata_closed(), "Escrow ATA should be closed");

        assert!(builder.is_escrow_closed(), "Escrow should be closed");
    }

    #[test]
    fn test_cancel_v2() {
        let deposit = 20u64;
        let seed = 123u64;
        let receive = 30u64;

        let builder = EscrowTestBuilder::new()
            .create_mints()
            .create_maker_ata_a()
            .mint_to_maker_ata_a(deposit)
            .set_escrow_accounts(seed)
            .execute_make_v2(deposit, seed, receive)
            .execute_cancel_v2();

        let maker_ata_a_data = builder.maker_ata_a_data();
        assert_eq!(maker_ata_a_data.amount(), deposit);
        assert_eq!(*maker_ata_a_data.owner(), builder.maker_pubkey());
        assert_eq!(*maker_ata_a_data.mint(), builder.mint_a());

        assert!(builder.is_escrow_ata_closed(), "Escrow ATA should be closed");

        assert!(builder.is_escrow_closed(), "Escrow should be closed");
    }
}
