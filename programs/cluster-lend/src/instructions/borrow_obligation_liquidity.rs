use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
    Accounts,
};
use anchor_spl::token::{self, Token, TokenAccount};

use crate::{
    check_refresh_ixs,
    errors::LendingError,
    gen_signer_seeds,
    lending_market::{lending_checks, lending_operations},
    state::{LendingMarket, Reserve},
    utils::{seeds, token_transfer},
    xmsg, CalculateBorrowResult, LendingAction, Obligation,
};

pub fn process_borrow_obligation_liquidity(
    ctx: Context<BorrowObligationLiquidityCtx>,
    liquidity_amount: u64,
) -> Result<()> {
    // check_refresh_ixs!(ctx, borrow_reserve);
    lending_checks::borrow_obligation_liquidity_checks(&ctx)?;

    let borrow_reserve = &mut ctx.accounts.borrow_reserve.load_mut()?;
    let lending_market = &ctx.accounts.lending_market.load()?;
    let obligation = &mut ctx.accounts.obligation.load_mut()?;
    let lending_market_key = ctx.accounts.lending_market.key();
    let clock = &Clock::get()?;

    let authority_signer_seeds =
        gen_signer_seeds!(lending_market_key.as_ref(), lending_market.bump as u8);

    let initial_reserve_token_balance =
        token::accessor::amount(&ctx.accounts.reserve_source_liquidity.to_account_info())?;
    let initial_reserve_available_liquidity = borrow_reserve.liquidity.available_amount;

    let CalculateBorrowResult {
        receive_amount,
        borrow_fee,
        ..
    } = lending_operations::borrow_obligation_liquidity(
        lending_market,
        borrow_reserve,
        obligation,
        liquidity_amount,
        clock,
        ctx.accounts.borrow_reserve.key(),
    )?;

    xmsg!("pnl: Borrow obligation liquidity {receive_amount} with borrow_fee {borrow_fee}",);

    if borrow_fee > 0 {
        token_transfer::send_origination_fees_transfer(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.reserve_source_liquidity.to_account_info(),
            ctx.accounts
                .borrow_reserve_liquidity_fee_receiver
                .to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            authority_signer_seeds,
            borrow_fee,
        )?;
    }

    token_transfer::borrow_obligation_liquidity_transfer(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.reserve_source_liquidity.to_account_info(),
        ctx.accounts.user_destination_liquidity.to_account_info(),
        ctx.accounts.lending_market_authority.to_account_info(),
        authority_signer_seeds,
        receive_amount,
    )?;

    lending_checks::post_transfer_vault_balance_liquidity_reserve_checks(
        token::accessor::amount(&ctx.accounts.reserve_source_liquidity.to_account_info()).unwrap(),
        borrow_reserve.liquidity.available_amount,
        initial_reserve_token_balance,
        initial_reserve_available_liquidity,
        LendingAction::Subtractive(borrow_fee + receive_amount),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct BorrowObligationLiquidityCtx<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = lending_market,
        has_one = owner @ LendingError::InvalidObligationOwner
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
    pub borrow_reserve: AccountLoader<'info, Reserve>,

    #[account(mut,
        address = borrow_reserve.load()?.liquidity.supply_vault
    )]
    pub reserve_source_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        address = borrow_reserve.load()?.liquidity.fee_vault
    )]
    pub borrow_reserve_liquidity_fee_receiver: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        token::mint = reserve_source_liquidity.mint,
        token::authority = owner
    )]
    pub user_destination_liquidity: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    /// CHECK: instruction_sysvar account
    #[account(address = SysInstructions::id())]
    pub instruction_sysvar_account: AccountInfo<'info>,
}
