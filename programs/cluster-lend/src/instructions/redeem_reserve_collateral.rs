use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
    Accounts,
};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::{
    gen_signer_seeds,
    lending_market::{
        post_transfer_vault_balance_liquidity_reserve_checks, redeem_reserve_collateral,
        redeem_reserve_collateral_checks, refresh_reserve,
    },
    state::{LendingAction, LendingMarket, RedeemReserveCollateralAccounts, Reserve},
    utils::{seeds, token_transfer}, xmsg,
};

pub fn process_redeem_reserve_collateral(
    ctx: Context<RedeemReserveCollateralCtx>,
    collateral_amount: u64,
) -> Result<()> {
    redeem_reserve_collateral_checks(&RedeemReserveCollateralAccounts {
        user_source_collateral: ctx.accounts.user_source_collateral.clone(),
        user_destination_liquidity: ctx.accounts.user_destination_liquidity.clone(),
        reserve: ctx.accounts.reserve.clone(),
        reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.clone(),
        reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.clone(),
        lending_market: ctx.accounts.lending_market.clone(),
        lending_market_authority: ctx.accounts.lending_market_authority.clone(),
        owner: ctx.accounts.owner.clone(),
        token_program: ctx.accounts.token_program.clone(),
    })?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let lending_market = &ctx.accounts.lending_market.load()?;
    let clock = Clock::get()?;

    let lending_market_key = ctx.accounts.lending_market.key();
    let authority_signer_seeds =
        gen_signer_seeds!(lending_market_key.as_ref(), lending_market.bump as u8);

    let initial_reserve_token_balance =
        token::accessor::amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;
    let initial_reserve_available_liquidity = reserve.liquidity.available_amount;

    refresh_reserve(reserve, &clock, None)?;
    let withdraw_liquidity_amount =
        redeem_reserve_collateral(reserve, collateral_amount, &clock, true)?;

    xmsg!(
        "pnl: Redeeming reserve collateral {}",
        withdraw_liquidity_amount
    );

    token_transfer::redeem_reserve_collateral_transfer(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.reserve_collateral_mint.to_account_info(),
        ctx.accounts.user_source_collateral.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.reserve_liquidity_supply.to_account_info(),
        ctx.accounts.user_destination_liquidity.to_account_info(),
        ctx.accounts.lending_market_authority.clone(),
        authority_signer_seeds,
        collateral_amount,
        withdraw_liquidity_amount,
    )?;

    post_transfer_vault_balance_liquidity_reserve_checks(
        token::accessor::amount(&ctx.accounts.reserve_liquidity_supply.to_account_info()).unwrap(),
        reserve.liquidity.available_amount,
        initial_reserve_token_balance,
        initial_reserve_available_liquidity,
        LendingAction::Subtractive(withdraw_liquidity_amount),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct RedeemReserveCollateralCtx<'info> {
    pub owner: Signer<'info>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    #[account(mut,
        has_one = lending_market
    )]
    pub reserve: AccountLoader<'info, Reserve>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut,
        address = reserve.load()?.collateral.mint_pubkey
    )]
    pub reserve_collateral_mint: Box<Account<'info, Mint>>,
    #[account(mut,
        address = reserve.load()?.liquidity.supply_vault
    )]
    pub reserve_liquidity_supply: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        token::mint = reserve_collateral_mint
    )]
    pub user_source_collateral: Box<Account<'info, TokenAccount>>,
    #[account(mut,
        token::mint = reserve.load()?.liquidity.mint_pubkey
    )]
    pub user_destination_liquidity: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    /// CHECK: instruction_sysvar account
    #[account(address = SysInstructions::id())]
    pub instruction_sysvar_account: AccountInfo<'info>,
}
