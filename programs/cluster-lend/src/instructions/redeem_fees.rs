use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    gen_signer_seeds, lending_market::lending_operations, utils::{seeds, token_transfer}, xmsg, LendingMarket, Reserve
};

pub fn process_redeem_fees(ctx: Context<RedeemFeesCtx>) -> Result<()> {
    let clock = &Clock::get()?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let lending_market = &ctx.accounts.lending_market.load()?;
    let lending_market_key = ctx.accounts.lending_market.key();

    let authority_signer_seeds = gen_signer_seeds!(lending_market_key, lending_market.bump as u8);

    let withdraw_amount = lending_operations::redeem_fees(reserve, clock.slot)?;

    xmsg!("Redeeming fees: {}", withdraw_amount);

    token_transfer::withdraw_fees_from_reserve(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.reserve_supply_liquidity.to_account_info(),
        ctx.accounts
            .reserve_liquidity_fee_receiver
            .to_account_info(),
        ctx.accounts.lending_market_authority.to_account_info(),
        authority_signer_seeds,
        withdraw_amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct RedeemFeesCtx<'info> {
    #[account(mut,
        has_one = lending_market)]
    pub reserve: AccountLoader<'info, Reserve>,

    #[account(mut, address = reserve.load()?.liquidity.fee_vault)]
    pub reserve_liquidity_fee_receiver: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = reserve.load()?.liquidity.supply_vault)]
    pub reserve_supply_liquidity: Box<Account<'info, TokenAccount>>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
