#[cfg(test)]
mod helpers;
use std::rc::Rc;

use anchor_lang::AnchorSerialize;
use cluster_lend::{
    errors::LendingError, LendingMarket, Reserve, ReserveStatus, UpdateConfigMode,
    UpdateLendingMarketMode,
};
use lending_market::LendingMarketFixture;

use reserve::ReserveFixture;
use solana_program_test::*;

use helpers::*;
use solana_sdk::{
    clock::{self, Clock},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use test::{
    TestFixture, PYTH_SOL_FEED, SOL_MINT_DECIMALS, SOL_QUOTE_CURRENCY, TEST_RESERVE_CONFIG,
    USDC_QUOTE_CURRENCY,
};

#[tokio::test]
async fn test_success_new() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation, _) = scenario_1(
        &ReserveConfig {
            optimal_borrow_rate: 0,
            max_borrow_rate: 0,
            fees: ReserveFees::default(),
            ..test_reserve_config()
        },
        &test_reserve_config(),
    )
    .await;

    let liquidator = User::new_with_balances(
        &mut test,
        &[
            (&wsol_mint::id(), 100 * LAMPORTS_TO_SOL),
            (&usdc_reserve.account.collateral.mint_pubkey, 0),
            (&usdc_mint::id(), 0),
        ],
    )
    .await;

    let balance_checker = BalanceChecker::start(
        &mut test,
        &[
            &usdc_reserve,
            &user,
            &wsol_reserve,
            &usdc_reserve,
            &liquidator,
        ],
    )
    .await;

    // close LTV is 0.55, we've deposited 100k USDC and borrowed 10 SOL.
    // obligation gets liquidated if 100k * 0.55 = 10 SOL * sol_price => sol_price = 5.5k
    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 5500,
            conf: 0,
            expo: 0,
            ema_price: 5500,
            ema_conf: 0,
        },
    )
    .await;

    lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &wsol_reserve,
            &usdc_reserve,
            &obligation,
            &liquidator,
            u64::MAX,
        )
        .await
        .unwrap();

    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;

    // 55k * 0.2 => 11k worth of SOL gets repaid
    // => 11k worth of USDC gets withdrawn + bonus.
    // bonus is 5%:
    // - 1% protocol liquidation fee: 110
    // - 4% liquidator bonus: 440
    let bonus = (usdc_reserve.account.config.liquidation_bonus
        + usdc_reserve.account.config.protocol_liquidation_fee / 10) as u64;

    let expected_borrow_repaid = 10 * (LIQUIDATION_CLOSE_FACTOR as u64) / 100;
    let expected_usdc_withdrawn = expected_borrow_repaid * 5500 * (100 + bonus) / 100;

    let expected_protocol_liquidation_fee = 110;

    let expected_balance_changes = HashSet::from([
        // liquidator
        TokenBalanceChange {
            token_account: liquidator.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: ((expected_usdc_withdrawn - expected_protocol_liquidation_fee)
                * FRACTIONAL_TO_USDC) as i128,
        },
        TokenBalanceChange {
            token_account: liquidator.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -((expected_borrow_repaid * LAMPORTS_TO_SOL) as i128),
        },
        // usdc reserve
        TokenBalanceChange {
            token_account: usdc_reserve.account.collateral.supply_pubkey,
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.liquidity.supply_pubkey,
            mint: usdc_mint::id(),
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.config.fee_receiver,
            mint: usdc_mint::id(),
            diff: (expected_protocol_liquidation_fee * FRACTIONAL_TO_USDC) as i128,
        },
        // wsol reserve
        TokenBalanceChange {
            token_account: wsol_reserve.account.liquidity.supply_pubkey,
            mint: wsol_mint::id(),
            diff: (expected_borrow_repaid * LAMPORTS_TO_SOL) as i128,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128)
        }])
    );

    // check program state
    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    assert_eq!(lending_market_post.account, lending_market.account);

    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            liquidity: ReserveLiquidity {
                available_amount: usdc_reserve.account.liquidity.available_amount
                    - expected_usdc_withdrawn * FRACTIONAL_TO_USDC,
                ..usdc_reserve.account.liquidity
            },
            collateral: ReserveCollateral {
                mint_total_supply: usdc_reserve.account.collateral.mint_total_supply
                    - expected_usdc_withdrawn * FRACTIONAL_TO_USDC,
                ..usdc_reserve.account.collateral
            },
            attributed_borrow_value: Decimal::from(55000u64),
            ..usdc_reserve.account
        }
    );

    let wsol_reserve_post = test.load_account::<Reserve>(wsol_reserve.pubkey).await;
    assert_eq!(
        wsol_reserve_post.account,
        Reserve {
            liquidity: ReserveLiquidity {
                available_amount: wsol_reserve.account.liquidity.available_amount
                    + expected_borrow_repaid * LAMPORTS_TO_SOL,
                borrowed_amount_wads: wsol_reserve
                    .account
                    .liquidity
                    .borrowed_amount_wads
                    .try_sub(Decimal::from(expected_borrow_repaid * LAMPORTS_TO_SOL))
                    .unwrap(),
                market_price: Decimal::from(5500u64),
                smoothed_market_price: Decimal::from(5500u64),
                ..wsol_reserve.account.liquidity
            },
            ..wsol_reserve.account
        }
    );

    let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;
    assert_eq!(
        obligation_post.account,
        Obligation {
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            deposits: [ObligationCollateral {
                deposit_reserve: usdc_reserve.pubkey,
                deposited_amount: (100_000 - expected_usdc_withdrawn) * FRACTIONAL_TO_USDC,
                market_value: Decimal::from(100_000u64), // old value
                attributed_borrow_value: obligation_post.account.deposits[0]
                    .attributed_borrow_value, // don't care about verifying this here
            }]
            .to_vec(),
            borrows: [ObligationLiquidity {
                borrow_reserve: wsol_reserve.pubkey,
                cumulative_borrow_rate_wads: Decimal::one(),
                borrowed_amount_wads: Decimal::from(10 * LAMPORTS_TO_SOL)
                    .try_sub(Decimal::from(expected_borrow_repaid * LAMPORTS_TO_SOL))
                    .unwrap(),
                market_value: Decimal::from(55_000u64),
            }]
            .to_vec(),
            deposited_value: Decimal::from(100_000u64),
            borrowed_value: Decimal::from(55_000u64),
            unweighted_borrowed_value: Decimal::from(55_000u64),
            borrowed_value_upper_bound: Decimal::from(55_000u64),
            allowed_borrow_value: Decimal::from(50_000u64),
            unhealthy_borrow_value: Decimal::from(55_000u64),
            ..obligation.account
        }
    );
}