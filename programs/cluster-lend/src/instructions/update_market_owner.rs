use anchor_lang::prelude::*;

use crate::state::LendingMarket;

pub fn process_update_market_owner(ctx: Context<UpdateMarketOwnerCtx>) -> Result<()> {
    let market = &mut ctx.accounts.lending_market.load_mut()?;
    market.owner = ctx.accounts.new_owner.key();

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateMarketOwnerCtx<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: new owner account
    pub new_owner: UncheckedAccount<'info>,

    #[account(mut,
        has_one = owner,
    )]
    pub lending_market: AccountLoader<'info, LendingMarket>,

    pub system_program: Program<'info, System>,
}
