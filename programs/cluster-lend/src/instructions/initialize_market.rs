use anchor_lang::prelude::*;

use crate::{
    state::{InitLendingMarketParams, LendingMarket},
    utils::seeds,
};

pub fn process_initialize_market(
    ctx: Context<InitializeMarketCtx>,
    quote_currency: [u8; 32],
) -> Result<()> {
    let lending_market = &mut ctx.accounts.lending_market.load_init()?;

    lending_market.init(InitLendingMarketParams {
        quote_currency,
        owner: ctx.accounts.owner.key(),
        bump: ctx.bumps.lending_market_authority,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarketCtx<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + std::mem::size_of::<LendingMarket>()
    )]
    pub lending_market: AccountLoader<'info, LendingMarket>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump
    )]
    pub lending_market_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
