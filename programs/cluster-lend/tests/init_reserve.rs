#[cfg(test)]
mod helpers;

use cluster_lend::{Reserve, ReserveStatus, UpdateConfigMode};
use lending_market::LendingMarketFixture;

use reserve::ReserveFixture;
use solana_program_test::*;

use helpers::*;
use solana_sdk::{signature::Keypair, signer::Signer};
use test::{TestFixture, PYTH_SOL_FEED, TEST_RESERVE_CONFIG, USDC_QUOTE_CURRENCY};

#[tokio::test]
async fn success_init_update_reserve() {
    let test_f = TestFixture::new().await;

    let payer = test_f.payer_keypair();

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: payer.pubkey(),
    };

    let reserve_key = Keypair::new();
    let reserve_f = ReserveFixture {
        key: reserve_key.pubkey(),
        owner: payer.pubkey(),
        payer: payer.pubkey(),
        lending_market: lending_market_f.key,
        liquidity_mint: test_f.usdc_mint.key,
    };

    let r = test_f
        .send_transaction(
            &[
                lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                reserve_f.initialize_reserve_ix(),
            ],
            &[&payer, &lending_market_key, &reserve_key],
        )
        .await;
    assert!(r.is_ok());

    // Fetch reserve account
    let reserve: Reserve = test_f.load_and_deserialize(&reserve_f.key).await;

    // Check properties
    assert_eq!(reserve.lending_market, lending_market_key.pubkey());
    assert_eq!(reserve.config.status(), ReserveStatus::Hidden);

    // Test as entire config update
    let r = test_f
        .send_transaction(
            &[
                reserve_f.update_reserve_ix(TEST_RESERVE_CONFIG),
                reserve_f.refresh_ix(Some(PYTH_SOL_FEED)),
            ],
            &[&payer],
        )
        .await;
    assert!(r.is_ok());

    let reserve: Reserve = test_f.load_and_deserialize(&reserve_f.key).await;
    assert_eq!(reserve.config.status(), ReserveStatus::Active);
    assert_eq!(
        reserve.config.deleveraging_margin_call_period_secs,
        TEST_RESERVE_CONFIG.deleveraging_margin_call_period_secs
    );

    // Test as individual field
    let mut value: [u8; 32] = [0; 32];
    value[0] = 32;
    let r = test_f
        .send_transaction(
            &[reserve_f
                .update_reserve_mode_ix(UpdateConfigMode::UpdateLoanToValuePct as u64, value)],
            &[&payer],
        )
        .await;
    assert!(r.is_ok());

    let reserve: Reserve = test_f.load_and_deserialize(&reserve_f.key).await;
    assert_eq!(reserve.config.loan_to_value_pct, 32);
}
