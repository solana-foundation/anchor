use anchor_lang::AccountDeserialize;
use anchor_test_utils::{anchor_instruction, TestContext};
use anyhow::Result;
use mpl_token_metadata::{
    accounts::Metadata, instructions::CreateMetadataAccountV3InstructionArgs, types::DataV2,
};
use solana_account::Account;
use solana_keypair::{Keypair, Signer};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_sysvar::rent::Rent;
use spl_token_interface::state::Account as TokenAccount;
use std::path::Path;

const PREFIX: &[u8] = b"auction_house";
const FEE_PAYER: &[u8] = b"fee_payer";
const TREASURY: &[u8] = b"treasury";
const SIGNER: &[u8] = b"signer";
const ZERO: [u8; 8] = [0; 8];

fn create_token_metadata_account(
    ctx: &mut TestContext,
    mint: Pubkey,
    args: CreateMetadataAccountV3InstructionArgs,
) -> Result<Pubkey> {
    let metadata = Metadata::find_pda(&mint).0;
    let authority = ctx.payer_pubkey();
    let metadata_state = Metadata {
        key: mpl_token_metadata::types::Key::MetadataV1,
        update_authority: authority,
        mint,
        name: args.data.name,
        symbol: args.data.symbol,
        uri: args.data.uri,
        seller_fee_basis_points: args.data.seller_fee_basis_points,
        creators: args.data.creators,
        primary_sale_happened: false,
        is_mutable: args.is_mutable,
        edition_nonce: None,
        token_standard: None,
        collection: args.data.collection,
        uses: args.data.uses,
        collection_details: args.collection_details,
        programmable_config: None,
    };
    let data = borsh::to_vec(&metadata_state)?;
    let rent = Rent::default();
    let account = Account {
        lamports: rent.minimum_balance(data.len()),
        data,
        owner: mpl_token_metadata::ID,
        executable: false,
        rent_epoch: 0,
    };
    ctx.svm().set_account(metadata, account)?;
    Ok(metadata)
}

fn send_ix(ctx: &mut TestContext, ix: solana_transaction::Instruction, signers: &[&Keypair]) {
    ctx.svm().expire_blockhash();
    let tx_result = ctx.send_signed_transaction_with_payer(&[ix], signers, true);
    assert!(tx_result.is_ok(), "{tx_result:?}");
}

#[test]
fn test_auction_house() {
    let mut ctx = TestContext::new();
    let authority = ctx.payer_pubkey();
    ctx.airdrop_payer(1_000_000_000).unwrap();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    ctx.build_and_deploy_sbf_program(
        manifest_dir.join("Cargo.toml"),
        "auction_house",
        auction_house::ID,
    )
    .unwrap();

    let metadata_program = manifest_dir.join("prebuilt-programs/metaplex_token_metadata.so");
    ctx.svm()
        .add_program_from_file(mpl_token_metadata::ID, metadata_program)
        .unwrap();

    let treasury_mint = spl_token_interface::native_mint::ID;
    ctx.init_native_mint().unwrap();
    let buyer = Keypair::new();
    let seller = Keypair::new();
    let buyer_price = 2_000_000_000u64;
    let token_size = 1u64;

    // Creates an NFT mint
    let mint = ctx
        .create_mint_builder()
        .decimals(6)
        .mint_authority(authority)
        .create()
        .unwrap();

    let metadata = create_token_metadata_account(
        &mut ctx,
        mint,
        CreateMetadataAccountV3InstructionArgs {
            data: DataV2 {
                name: "test-nft".to_string(),
                symbol: "TEST".to_string(),
                uri: "https://nothing.com".to_string(),
                seller_fee_basis_points: 1,
                creators: None,
                collection: None,
                uses: None,
            },
            is_mutable: true,
            collection_details: None,
        },
    )
    .unwrap();

    // Creates token accounts for the NFT
    let buyer_token_account = ctx
        .create_associated_token_account_builder(mint, buyer.pubkey())
        .create()
        .unwrap();

    let seller_token_account = ctx
        .create_associated_token_account_builder(mint, seller.pubkey())
        .amount(1)
        .create()
        .unwrap();

    // Initializes constants
    let (auction_house, _) = Pubkey::find_program_address(
        &[PREFIX, authority.as_ref(), treasury_mint.as_ref()],
        &auction_house::ID,
    );
    let (auction_house_fee_account, _) = Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), FEE_PAYER],
        &auction_house::ID,
    );
    let (auction_house_treasury, _) = Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), TREASURY],
        &auction_house::ID,
    );
    let (buyer_escrow, _) = Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), buyer.pubkey().as_ref()],
        &auction_house::ID,
    );
    let (program_as_signer, _) =
        Pubkey::find_program_address(&[PREFIX, SIGNER], &auction_house::ID);

    let buyer_price_bytes = buyer_price.to_le_bytes();
    let token_size_bytes = token_size.to_le_bytes();
    let (seller_trade_state, _) = Pubkey::find_program_address(
        &[
            PREFIX,
            seller.pubkey().as_ref(),
            auction_house.as_ref(),
            ctx.associated_token_address(seller.pubkey(), mint).as_ref(),
            treasury_mint.as_ref(),
            mint.as_ref(),
            buyer_price_bytes.as_ref(),
            token_size_bytes.as_ref(),
        ],
        &auction_house::ID,
    );
    let (free_seller_trade_state, _) = Pubkey::find_program_address(
        &[
            PREFIX,
            seller.pubkey().as_ref(),
            auction_house.as_ref(),
            ctx.associated_token_address(seller.pubkey(), mint).as_ref(),
            treasury_mint.as_ref(),
            mint.as_ref(),
            ZERO.as_ref(),
            token_size_bytes.as_ref(),
        ],
        &auction_house::ID,
    );
    let (buyer_trade_state, _) = Pubkey::find_program_address(
        &[
            PREFIX,
            buyer.pubkey().as_ref(),
            auction_house.as_ref(),
            ctx.associated_token_address(seller.pubkey(), mint).as_ref(),
            treasury_mint.as_ref(),
            mint.as_ref(),
            buyer_price_bytes.as_ref(),
            token_size_bytes.as_ref(),
        ],
        &auction_house::ID,
    );
    let free_trade_state_account = Account {
        lamports: Rent::default().minimum_balance(auction_house::TRADE_STATE_SIZE),
        data: vec![0; auction_house::TRADE_STATE_SIZE],
        owner: auction_house::ID,
        executable: false,
        rent_epoch: 0,
    };
    ctx.svm()
        .set_account(free_seller_trade_state, free_trade_state_account)
        .unwrap();

    // Funds the buyer with lamports so that it can bid
    ctx.airdrop(buyer.pubkey(), 20_000_000_000).unwrap();
    ctx.airdrop(seller.pubkey(), 20_000_000_000).unwrap();
    ctx.airdrop(auction_house_fee_account, 100_000_000_000)
        .unwrap();

    // Creates an auction house
    let create_auction_house_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::CreateAuctionHouse {
            treasury_mint,
            payer: authority,
            authority,
            fee_withdrawal_destination: authority,
            treasury_withdrawal_destination_owner: authority,
            treasury_withdrawal_destination: authority,
            auction_house,
            auction_house_fee_account,
            auction_house_treasury,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            rent: solana_sysvar::rent::ID,
        },
        auction_house::instruction::CreateAuctionHouse {
            seller_fee_basis_points: 1,
            requires_sign_off: true,
            can_change_sale_price: true,
        },
    );
    send_ix(&mut ctx, create_auction_house_ix, &[]);

    // Deposits into an escrow account
    let deposit_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Deposit {
            wallet: buyer.pubkey(),
            payment_account: buyer.pubkey(),
            transfer_authority: buyer.pubkey(),
            auction_house,
            escrow_payment_account: buyer_escrow,
            treasury_mint,
            authority,
            auction_house_fee_account,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            rent: solana_sysvar::rent::ID,
        },
        auction_house::instruction::Deposit {
            amount: 10_000_000_000,
        },
    );
    send_ix(&mut ctx, deposit_ix, &[&buyer]);

    // Withdraws from an escrow account
    let withdraw_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Withdraw {
            wallet: buyer.pubkey(),
            receipt_account: buyer.pubkey(),
            auction_house,
            auction_house_fee_account,
            escrow_payment_account: buyer_escrow,
            treasury_mint,
            authority,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        },
        auction_house::instruction::Withdraw {
            amount: 10_000_000_000,
        },
    );
    send_ix(&mut ctx, withdraw_ix, &[]);

    // Posts an offer
    let sell_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Sell {
            wallet: seller.pubkey(),
            token_account: seller_token_account,
            metadata,
            authority,
            auction_house,
            auction_house_fee_account,
            seller_trade_state,
            free_seller_trade_state,
            treasury_mint,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            program_as_signer,
            rent: solana_sysvar::rent::ID,
        },
        auction_house::instruction::Sell {
            buyer_price,
            token_size,
        },
    );
    send_ix(&mut ctx, sell_ix, &[]);

    // Cancels an offer
    let cancel_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Cancel {
            wallet: seller.pubkey(),
            token_account: seller_token_account,
            authority,
            treasury_mint,
            auction_house,
            auction_house_fee_account,
            trade_state: seller_trade_state,
            token_program: anchor_test_utils::token::ID,
        },
        auction_house::instruction::Cancel {
            _buyer_price: buyer_price,
            _token_size: token_size,
        },
    );
    send_ix(&mut ctx, cancel_ix, &[]);

    // Posts an offer (again)
    let sell_again_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Sell {
            wallet: seller.pubkey(),
            token_account: seller_token_account,
            metadata,
            authority,
            auction_house,
            auction_house_fee_account,
            seller_trade_state,
            free_seller_trade_state,
            treasury_mint,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            program_as_signer,
            rent: solana_sysvar::rent::ID,
        },
        auction_house::instruction::Sell {
            buyer_price,
            token_size,
        },
    );
    send_ix(&mut ctx, sell_again_ix, &[]);

    // Posts a bid
    let buy_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::Buy {
            wallet: buyer.pubkey(),
            payment_account: buyer.pubkey(),
            transfer_authority: buyer.pubkey(),
            treasury_mint,
            token_account: seller_token_account,
            metadata,
            authority,
            auction_house,
            auction_house_fee_account,
            buyer_trade_state,
            escrow_payment_account: buyer_escrow,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            rent: solana_sysvar::rent::ID,
        },
        auction_house::instruction::Buy {
            buyer_price,
            token_size,
        },
    );
    send_ix(&mut ctx, buy_ix, &[&buyer]);

    ctx.set_token_account_delegate(seller_token_account, Some(program_as_signer), token_size)
        .unwrap();

    // Executes a trade
    ctx.airdrop(auction_house_treasury, 890_880).unwrap();
    let before_escrow_lamports = ctx.svm().get_account(&buyer_escrow).unwrap().lamports;
    let before_seller_lamports = ctx.svm().get_account(&seller.pubkey()).unwrap().lamports;

    let execute_sale_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::ExecuteSale {
            buyer: buyer.pubkey(),
            seller: seller.pubkey(),
            token_account: seller_token_account,
            token_mint: mint,
            metadata,
            treasury_mint,
            seller_payment_receipt_account: seller.pubkey(),
            buyer_receipt_token_account: buyer_token_account,
            authority,
            auction_house,
            auction_house_fee_account,
            auction_house_treasury,
            escrow_payment_account: buyer_escrow,
            buyer_trade_state,
            seller_trade_state,
            free_trade_state: free_seller_trade_state,
            program_as_signer,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        },
        auction_house::instruction::ExecuteSale {
            buyer_price,
            token_size,
        },
    );
    send_ix(&mut ctx, execute_sale_ix, &[]);

    let after_escrow = ctx.svm().get_account(&buyer_escrow);
    let after_seller_lamports = ctx.svm().get_account(&seller.pubkey()).unwrap().lamports;
    assert!(after_escrow.is_none());
    assert_eq!(before_escrow_lamports, 2_000_000_000);
    assert_eq!(
        after_seller_lamports.saturating_sub(before_seller_lamports),
        1_999_800_000
    );

    let buyer_token_data = &ctx.svm().get_account(&buyer_token_account).unwrap().data;
    let buyer_token_state = TokenAccount::unpack(buyer_token_data).unwrap();
    assert_eq!(buyer_token_state.amount, 1);

    // Withdraws from the fee account
    let withdraw_fee_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::WithdrawFromFee {
            authority,
            treasury_mint,
            fee_withdrawal_destination: authority,
            auction_house,
            auction_house_fee_account,
            system_program: solana_sdk_ids::system_program::ID,
        },
        auction_house::instruction::WithdrawFromFee { amount: 1 },
    );
    send_ix(&mut ctx, withdraw_fee_ix, &[]);

    // Withdraws from the treasury account
    let withdraw_treasury_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::WithdrawFromTreasury {
            treasury_mint,
            authority,
            treasury_withdrawal_destination: authority,
            auction_house,
            auction_house_treasury,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
        },
        auction_house::instruction::WithdrawFromTreasury { amount: 1 },
    );
    send_ix(&mut ctx, withdraw_treasury_ix, &[]);

    // Updates an auction house
    let update_auction_house_ix = anchor_instruction(
        auction_house::ID,
        auction_house::accounts::UpdateAuctionHouse {
            treasury_mint,
            payer: authority,
            authority,
            new_authority: authority,
            fee_withdrawal_destination: authority,
            treasury_withdrawal_destination: authority,
            treasury_withdrawal_destination_owner: authority,
            auction_house,
            token_program: anchor_test_utils::token::ID,
            system_program: solana_sdk_ids::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        },
        auction_house::instruction::UpdateAuctionHouse {
            seller_fee_basis_points: Some(2),
            requires_sign_off: Some(true),
            can_change_sale_price: None,
        },
    );
    send_ix(&mut ctx, update_auction_house_ix, &[]);

    let ah_data = &ctx.svm().get_account(&auction_house).unwrap().data;
    let mut ah_data_slice: &[u8] = ah_data;
    let ah_account = auction_house::AuctionHouse::try_deserialize(&mut ah_data_slice).unwrap();
    assert_eq!(ah_account.seller_fee_basis_points, 2);
}
