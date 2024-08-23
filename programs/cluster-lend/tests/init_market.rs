#[cfg(test)]
mod helpers;
use std::rc::Rc;

use cluster_lend::{LendingMarket, UpdateLendingMarketMode};
use lending_market::LendingMarketFixture;

use solana_program_test::*;

use helpers::*;
use solana_sdk::{signature::Keypair, signer::Signer};
use test::{TestFixture, SOL_QUOTE_CURRENCY, USDC_QUOTE_CURRENCY};

#[tokio::test]
async fn success_init_lending_market() {
    let test_f = TestFixture::new().await;

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: test_f.payer(),
    };

    test_f
        .send_transaction(
            &[lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY)],
            &[&test_f.payer_keypair(), &lending_market_key],
        )
        .await
        .unwrap();

    // Fetch & deserialize lending_market account
    let lending_market: LendingMarket = test_f.load_and_deserialize(&lending_market_f.key).await;

    // Check properties
    assert_eq!(lending_market.quote_currency, USDC_QUOTE_CURRENCY);
    assert_eq!(lending_market.owner, test_f.payer());
}

#[tokio::test]
async fn success_update_lending_market() {
    let test_f = TestFixture::new().await;

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: test_f.payer(),
    };

    // update emergancy mode
    let mode = UpdateLendingMarketMode::UpdateEmergencyMode as u64;
    let mut value: [u8; 72] = [0; 72];
    value[0] = 1;

    let r = test_f
        .send_transaction(
            &[
                lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                lending_market_f.update_market_ix(mode, value),
            ],
            &[&test_f.payer_keypair(), &lending_market_key],
        )
        .await;
    assert!(r.is_ok());

    // Fetch & deserialize lending_market account
    let lending_market: LendingMarket = test_f.load_and_deserialize(&lending_market_f.key).await;

    // Check properties
    assert_eq!(lending_market.emergency_mode, 1);
}

#[tokio::test]
async fn success_update_lending_market_owner() {
    let test_f = TestFixture::new().await;

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: test_f.payer(),
    };

    let owner = test_f.payer_keypair();
    let new_owner = Keypair::new();

    let r = test_f
        .send_transaction(
            &[
                lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                lending_market_f.update_owner_ix(new_owner.pubkey()),
            ],
            &[&owner, &lending_market_key],
        )
        .await;
    assert!(r.is_ok());

    // Fetch & deserialize lending_market account
    let lending_market: LendingMarket = test_f.load_and_deserialize(&lending_market_f.key).await;

    // Check properties
    assert_eq!(lending_market.quote_currency, USDC_QUOTE_CURRENCY);
    assert_eq!(lending_market.owner, new_owner.pubkey());
}

#[tokio::test]
async fn failure_update_lending_market_with_invalid_owner() {
    let test_f = TestFixture::new().await;

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: test_f.payer(),
    };

    let owner = test_f.payer_keypair();
    let new_owner = Keypair::new();

    // update configure with invalid authority
    let mode = UpdateLendingMarketMode::UpdateBorrowingDisabled as u64;
    let mut value: [u8; 72] = [0; 72];
    value[0] = 1;

    let r = test_f
        .send_transaction(
            &[
                lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                lending_market_f.update_owner_ix(new_owner.pubkey()),
                lending_market_f.update_market_ix(mode, value),
            ],
            &[&owner, &lending_market_key],
        )
        .await;
    assert!(r.is_err());
}
