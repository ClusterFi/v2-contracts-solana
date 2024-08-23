#[cfg(test)]
mod helpers;
use std::rc::Rc;

use anchor_lang::{context, Key};
use anchor_spl::{
    associated_token::{create, get_associated_token_address, AssociatedToken},
    token::{self, spl_token::state::Account, TokenAccount},
};
use cluster_lend::{
    constants::ten_pow, errors::LendingError, utils::pda, InitObligationArgs, LendingMarket,
    Reserve, ReserveStatus, UpdateLendingMarketMode,
};
use lending_market::LendingMarketFixture;

use obligation::ObligationFixture;
use reserve::ReserveFixture;
use solana_program_test::*;

use helpers::*;
use solana_sdk::{
    clock::{self, Clock},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl::{MintFixture, TokenAccountFixture};
use test::{
    TestFixture, PYTH_SOL_FEED, SOL_MINT_DECIMALS, SOL_QUOTE_CURRENCY, TEST_RESERVE_CONFIG,
    USDC_MINT_DECIMALS, USDC_QUOTE_CURRENCY,
};

#[tokio::test]
async fn success_deposit_withdraw() {
    let user = Keypair::new();

    // setup market & reserve/obligation
    let test_f = TestFixture::new().await;
    let liquidity_mint_f = test_f.usdc_mint.clone();
    let (market_f, reserve_f, obligation_f) = test_f.setup(&user, &liquidity_mint_f.key).await;
    let reserve_pdas = pda::init_reserve_pdas(&market_f.key, &liquidity_mint_f.key);

    // supply test token
    let mut user_liquidity_balance = 1_000_000_000;
    let user_liquidity_ata = liquidity_mint_f
        .create_token_account_and_mint_to(&user, user_liquidity_balance)
        .await;

    // deposit obligation
    let deposit_amount = 1_000_000;
    let r = test_f
        .send_transaction(
            &[
                obligation_f.deposit_liquidity_collateral_ix(
                    deposit_amount,
                    &reserve_f,
                    user_liquidity_ata.key,
                ),
                reserve_f.refresh_ix(Some(PYTH_SOL_FEED)),
                obligation_f.refresh_ix(vec![reserve_f.key]),
            ],
            &[&user],
        )
        .await;
    assert!(r.is_ok());

    // check user's balance
    let user_ata: TokenAccount = test_f.load_and_deserialize(&user_liquidity_ata.key).await;
    assert_eq!(user_ata.amount, user_liquidity_balance - deposit_amount);
    user_liquidity_balance = user_liquidity_balance - deposit_amount;

    // check liquidity vault balance
    let reseve_supply_vault: TokenAccount = test_f
        .load_and_deserialize(&reserve_pdas.liquidity_supply_vault.key())
        .await;
    assert_eq!(deposit_amount, reseve_supply_vault.amount);

    // check collateral vault balance
    let mut collateral_balance = deposit_amount;
    let collateral_supply_vault: TokenAccount = test_f
        .load_and_deserialize(&&reserve_pdas.collateral_supply_vault.key())
        .await;
    assert_eq!(collateral_balance, collateral_supply_vault.amount);

    // withdraw obligation
    let user_collateral_ata_f = TokenAccountFixture::new(
        Rc::clone(&test_f.context),
        &reserve_pdas.collateral_ctoken_mint,
        &user.pubkey(),
    )
    .await;

    let withdraw_amount = 1000;
    let r = test_f
        .send_transaction(
            &[
                obligation_f.withdraw_collateral_ix(
                    withdraw_amount,
                    &reserve_f,
                    user_collateral_ata_f.key,
                ),
                obligation_f.refresh_ix(vec![reserve_f.key]),
                reserve_f.refresh_ix(Some(PYTH_SOL_FEED)),
            ],
            &[&user],
        )
        .await;
    assert!(r.is_ok());

    // check collateral vault balance
    collateral_balance -= withdraw_amount;
    let collateral_supply_vault: TokenAccount = test_f
        .load_and_deserialize(&reserve_pdas.collateral_supply_vault.key())
        .await;
    assert_eq!(collateral_balance, collateral_supply_vault.amount);
}
