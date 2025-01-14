use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::{
    check_refresh_ixs, gen_signer_seeds,
    lending_market::{lending_checks, lending_operations},
    utils::{seeds, token_transfer},
    xmsg, LendingAction, LendingMarket, LiquidateAndRedeemResult, Obligation,
    RedeemReserveCollateralAccounts, Reserve,
};

pub fn process_liquidate_obligation(
    ctx: Context<LiquidateObligationCtx>,
    liquidity_amount: u64,
    min_acceptable_received_collateral_amount: u64,
    max_allowed_ltv_override_percent: u64,
) -> Result<()> {
    xmsg!(
        "LiquidateObligation amount {} max_allowed_ltv_override_percent {}",
        liquidity_amount,
        max_allowed_ltv_override_percent
    );

    check_refresh_ixs!(ctx, withdraw_reserve, repay_reserve);

    lending_checks::liquidate_obligation_checks(&ctx)?;
    lending_checks::redeem_reserve_collateral_checks(&RedeemReserveCollateralAccounts {
        user_source_collateral: ctx.accounts.user_destination_collateral.clone(),
        user_destination_liquidity: ctx.accounts.user_destination_liquidity.clone(),
        reserve: ctx.accounts.withdraw_reserve.clone(),
        reserve_collateral_mint: ctx.accounts.withdraw_reserve_collateral_mint.clone(),
        reserve_liquidity_supply: ctx.accounts.withdraw_reserve_liquidity_supply.clone(),
        lending_market: ctx.accounts.lending_market.clone(),
        lending_market_authority: ctx.accounts.lending_market_authority.clone(),
        owner: ctx.accounts.liquidator.clone(),
        token_program: ctx.accounts.token_program.clone(),
    })?;

    let lending_market = &ctx.accounts.lending_market.load()?;
    let obligation = &mut ctx.accounts.obligation.load_mut()?;
    let lending_market_key = ctx.accounts.lending_market.key();
    let clock = &Clock::get()?;

    let max_allowed_ltv_override_pct_opt = if ctx.accounts.liquidator.key() == obligation.owner
        && max_allowed_ltv_override_percent > 0
    {
        if cfg!(feature = "staging") {
            Some(max_allowed_ltv_override_percent)
        } else {
            xmsg!("Warning! Attempting to set an ltv override outside the staging program");
            None
        }
    } else {
        None
    };

    let initial_withdraw_reserve_token_balance = token::accessor::amount(
        &ctx.accounts
            .withdraw_reserve_liquidity_supply
            .to_account_info(),
    )?;

    let initial_repay_reserve_token_balance = token::accessor::amount(
        &ctx.accounts
            .repay_reserve_liquidity_supply
            .to_account_info(),
    )?;

    let (initial_repay_reserve_available_amount, initial_withdraw_reserve_available_amount) =
        lending_checks::initial_liquidation_reserve_liquidity_available_amount(
            &ctx.accounts.repay_reserve,
            &ctx.accounts.withdraw_reserve,
        );

    let authority_signer_seeds = gen_signer_seeds!(lending_market_key, lending_market.bump as u8);

    let LiquidateAndRedeemResult {
        repay_amount,
        withdraw_collateral_amount,
        withdraw_amount,
        total_withdraw_liquidity_amount,
        ..
    } = lending_operations::liquidate_and_redeem(
        lending_market,
        &ctx.accounts.repay_reserve,
        &ctx.accounts.withdraw_reserve,
        obligation,
        clock,
        liquidity_amount,
        min_acceptable_received_collateral_amount,
        max_allowed_ltv_override_pct_opt,
    )?;

    token_transfer::repay_obligation_liquidity_transfer(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.user_source_liquidity.to_account_info(),
        ctx.accounts
            .repay_reserve_liquidity_supply
            .to_account_info(),
        ctx.accounts.liquidator.to_account_info(),
        repay_amount,
    )?;

    token_transfer::withdraw_obligation_collateral_transfer(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.user_destination_collateral.to_account_info(),
        ctx.accounts
            .withdraw_reserve_collateral_supply
            .to_account_info(),
        ctx.accounts.lending_market_authority.to_account_info(),
        authority_signer_seeds,
        withdraw_amount,
    )?;

    if let Some((withdraw_liquidity_amount, protocol_fee)) = total_withdraw_liquidity_amount {
        token_transfer::redeem_reserve_collateral_transfer(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts
                .withdraw_reserve_collateral_mint
                .to_account_info(),
            ctx.accounts.user_destination_collateral.to_account_info(),
            ctx.accounts.liquidator.to_account_info(),
            ctx.accounts
                .withdraw_reserve_liquidity_supply
                .to_account_info(),
            ctx.accounts.user_destination_liquidity.to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            authority_signer_seeds,
            withdraw_collateral_amount,
            withdraw_liquidity_amount,
        )?;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.user_destination_liquidity.to_account_info(),
                    to: ctx
                        .accounts
                        .withdraw_reserve_liquidity_fee_receiver
                        .to_account_info(),
                    authority: ctx.accounts.liquidator.to_account_info(),
                },
            ),
            protocol_fee,
        )?;
        let withdraw_reserve = &ctx.accounts.withdraw_reserve.load()?;

        let net_withdrawal_amount = if ctx
            .accounts
            .withdraw_reserve_liquidity_supply
            .to_account_info()
            .key
            == ctx
                .accounts
                .repay_reserve_liquidity_supply
                .to_account_info()
                .key
        {
            withdraw_liquidity_amount - repay_amount
        } else {
            withdraw_liquidity_amount
        };

        lending_checks::post_transfer_vault_balance_liquidity_reserve_checks(
            token::accessor::amount(
                &ctx.accounts
                    .withdraw_reserve_liquidity_supply
                    .to_account_info(),
            )
            .unwrap(),
            withdraw_reserve.liquidity.available_amount,
            initial_withdraw_reserve_token_balance,
            initial_withdraw_reserve_available_amount,
            LendingAction::Subtractive(net_withdrawal_amount),
        )?;
    }
    let repay_reserve = &ctx.accounts.repay_reserve.load()?;

    if ctx
        .accounts
        .withdraw_reserve_liquidity_supply
        .to_account_info()
        .key
        != ctx
            .accounts
            .repay_reserve_liquidity_supply
            .to_account_info()
            .key
        || total_withdraw_liquidity_amount.is_none()
    {
        lending_checks::post_transfer_vault_balance_liquidity_reserve_checks(
            token::accessor::amount(
                &ctx.accounts
                    .repay_reserve_liquidity_supply
                    .to_account_info(),
            )
            .unwrap(),
            repay_reserve.liquidity.available_amount,
            initial_repay_reserve_token_balance,
            initial_repay_reserve_available_amount,
            LendingAction::Additive(repay_amount),
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LiquidateObligationCtx<'info> {
    pub liquidator: Signer<'info>,

    #[account(mut,
        has_one = lending_market
    )]
    pub obligation: AccountLoader<'info, Obligation>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut,
        has_one = lending_market
    )]
    pub repay_reserve: AccountLoader<'info, Reserve>,

    #[account(mut,
        address = repay_reserve.load()?.liquidity.supply_vault
    )]
    pub repay_reserve_liquidity_supply: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        has_one = lending_market
    )]
    pub withdraw_reserve: AccountLoader<'info, Reserve>,
    #[account(mut,
        address = withdraw_reserve.load()?.collateral.mint_pubkey
    )]
    pub withdraw_reserve_collateral_mint: Box<Account<'info, Mint>>,
    #[account(mut,
        address = withdraw_reserve.load()?.collateral.supply_vault
    )]
    pub withdraw_reserve_collateral_supply: Box<Account<'info, TokenAccount>>,
    #[account(mut,
        address = withdraw_reserve.load()?.liquidity.supply_vault
    )]
    pub withdraw_reserve_liquidity_supply: Box<Account<'info, TokenAccount>>,
    #[account(mut,
        address = withdraw_reserve.load()?.liquidity.fee_vault
    )]
    pub withdraw_reserve_liquidity_fee_receiver: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub user_source_liquidity: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_destination_collateral: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_destination_liquidity: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    /// CHECK: instruction_sysvar account
    #[account(address = SysInstructions::id())]
    pub instruction_sysvar_account: AccountInfo<'info>,
}
