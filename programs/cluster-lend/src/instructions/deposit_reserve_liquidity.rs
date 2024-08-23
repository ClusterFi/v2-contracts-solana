use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
    Accounts,
};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::{
    errors::LendingError, gen_signer_seeds, lending_market::{
        deposit_reserve_liquidity, lending_checks,
        post_transfer_vault_balance_liquidity_reserve_checks, refresh_reserve,
    }, state::{LendingAction, LendingMarket, Reserve}, utils::{seeds, token_transfer}, xmsg, DepositReserveLiquidityAccounts
};

pub fn process_deposit_reserve_liquidity(
    ctx: Context<DepositReserveLiquidityCtx>,
    liquidity_amount: u64,
) -> Result<()> {
    require!(liquidity_amount != 0, LendingError::InvalidAmount);

    lending_checks::deposit_reserve_liquidity_checks(&DepositReserveLiquidityAccounts {
        lending_market: ctx.accounts.lending_market.clone(),
        lending_market_authority: ctx.accounts.lending_market_authority.clone(),
        reserve: ctx.accounts.reserve.clone(),
        reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.clone(),
        reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.clone(),
        owner: ctx.accounts.owner.clone(),
        user_source_liquidity: ctx.accounts.user_source_liquidity.clone(),
        user_destination_collateral: ctx.accounts.user_destination_collateral.clone(),
        token_program: ctx.accounts.token_program.clone(),
    })?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let lending_market = &ctx.accounts.lending_market.load()?;
    let clock = &Clock::get()?;

    refresh_reserve(reserve, &clock, None)?;

    let lending_market_key = ctx.accounts.lending_market.key();
    let authority_signer_seeds =
        gen_signer_seeds!(lending_market_key.as_ref(), lending_market.bump as u8);

    let initial_reserve_token_balance =
        token::accessor::amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;
    let initial_reserve_available_liquidity = reserve.liquidity.available_amount;
    let collateral_amount = deposit_reserve_liquidity(reserve, &clock, liquidity_amount)?;

    xmsg!(
        "pnl: Depositing in reserve {:?} liquidity {}",
        ctx.accounts.reserve.key(),
        liquidity_amount
    );

    token_transfer::deposit_reserve_liquidity_transfer(
        ctx.accounts.user_source_liquidity.to_account_info(),
        ctx.accounts.reserve_liquidity_supply.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.reserve_collateral_mint.to_account_info(),
        ctx.accounts.user_destination_collateral.to_account_info(),
        ctx.accounts.lending_market_authority.clone(),
        authority_signer_seeds,
        liquidity_amount,
        collateral_amount,
    )?;

    post_transfer_vault_balance_liquidity_reserve_checks(
        token::accessor::amount(&ctx.accounts.reserve_liquidity_supply.to_account_info()).unwrap(),
        reserve.liquidity.available_amount,
        initial_reserve_token_balance,
        initial_reserve_available_liquidity,
        LendingAction::Additive(liquidity_amount),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct DepositReserveLiquidityCtx<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = lending_market
    )]
    pub reserve: AccountLoader<'info, Reserve>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut, address = reserve.load()?.liquidity.supply_vault)]
    pub reserve_liquidity_supply: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = reserve.load()?.collateral.mint_pubkey)]
    pub reserve_collateral_mint: Box<Account<'info, Mint>>,

    #[account(mut,
        token::mint = reserve_liquidity_supply.mint
    )]
    pub user_source_liquidity: Box<Account<'info, TokenAccount>>,
    #[account(mut,
        token::mint = reserve_collateral_mint.key()
    )]
    pub user_destination_collateral: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    /// CHECK: instruction_sysvar account
    #[account(address = SysInstructions::id())]
    pub instruction_sysvar_account: AccountInfo<'info>,
}
