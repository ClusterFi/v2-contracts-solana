use anchor_lang::prelude::*;

use crate::{
    errors::LendingError,
    lending_market::lending_operations,
    state::{LendingMarket, Obligation, Reserve},
    utils::FatAccountLoader, xmsg,
};

pub fn process_refresh_obligation(ctx: Context<RefreshObligationCtx>) -> Result<()> {
    let obligation = &mut ctx.accounts.obligation.load_mut()?;
    let clock = &Clock::get()?;
    let lending_market = &ctx.accounts.lending_market.load()?;
    let borrow_count = obligation.borrows_count();
    let reserves_count = borrow_count + obligation.deposits_count();

    if ctx.remaining_accounts.iter().len() != reserves_count {
        xmsg!("expected_remaining_accounts={}", reserves_count,);
        return err!(LendingError::InvalidAccountInput);
    }

    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    lending_operations::refresh_obligation(obligation, lending_market, clock.slot, reserves_iter)?;

    Ok(())
}

#[derive(Accounts)]
pub struct RefreshObligationCtx<'info> {
    pub lending_market: AccountLoader<'info, LendingMarket>,

    #[account(mut, has_one = lending_market)]
    pub obligation: AccountLoader<'info, Obligation>,
}
