use solana_program_test::ProgramTest;
use solana_pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
async fn check_entrypoint() {
    let program_id = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let pt = ProgramTest::new(
        "solana_program_test_compatibility",
        program_id,
        None,
    );
    let _context = pt.start_with_context().await;
}
