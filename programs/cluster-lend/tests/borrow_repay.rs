#[cfg(test)]
mod helpers;
use cluster_lend::utils::pda;
use solana_program_test::*;

use helpers::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use test::{TestFixture, PYTH_USDC_FEED};

#[tokio::test]
async fn success_borrow_repay() {
    // create test user and supply test token
    let user1 = Keypair::new();

    // setup market & reserve/obligation
    let test_f = TestFixture::new().await;
    let usdc_mint_f = test_f.usdc_mint.clone();
    let (market_f, reserve_f, obligation_f) = test_f.setup(&user1, &usdc_mint_f.key).await;
    let reserve_pdas = pda::init_reserve_pdas(&market_f.key, &usdc_mint_f.key);

    // deposit liquidity & obligation
    let deposit_amount = 1_000_000;
    let user1_liquidity_ata = usdc_mint_f
        .create_token_account_and_mint_to(&user1, deposit_amount)
        .await;

    // Update price oracle
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;
    test_f.set_time(120);

    test_f
        .send_transaction(
            &[
                obligation_f.deposit_liquidity_collateral_ix(
                    deposit_amount,
                    &reserve_f,
                    user1_liquidity_ata.key,
                ),
                reserve_f.refresh_ix(Some(PYTH_USDC_FEED)),
                obligation_f.refresh_ix(vec![reserve_f.key]),
            ],
            &[&user1],
        )
        .await
        .unwrap();

    // borrow collateral
    let borrow_amount = 300_000;
    test_f
        .send_transaction(
            &[
                obligation_f.borrow_liquidity_ix(
                    borrow_amount,
                    &reserve_f,
                    user1_liquidity_ata.key,
                ),
                reserve_f.refresh_ix(Some(PYTH_USDC_FEED)),
                obligation_f.refresh_ix(vec![reserve_f.key]),
            ],
            &[&user1],
        )
        .await
        .unwrap();
}
