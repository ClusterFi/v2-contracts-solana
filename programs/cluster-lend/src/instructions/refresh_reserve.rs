use anchor_lang::prelude::*;

use crate::{
    constants::PROGRAM_VERSION,
    errors::LendingError,
    lending_market::lending_operations,
    state::{LendingMarket, Reserve},
    utils::{constraints, get_price}, xmsg,
};

pub fn process_refresh_reserve(ctx: Context<RefreshReserveCtx>) -> Result<()> {
    let clock = &Clock::get()?;
    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let lending_market = &ctx.accounts.lending_market.load()?;

    constraints::check_remaining_accounts(&ctx)?;

    require!(
        reserve.version == PROGRAM_VERSION as u64,
        LendingError::ReserveDeprecated
    );

    let price_res = if lending_operations::is_price_refresh_needed(
        reserve,
        lending_market,
        clock.unix_timestamp,
    ) {
        reserve
            .config
            .token_info
            .validate_token_info_config(&ctx.accounts.pyth_oracle)?;

        get_price(
            &reserve.config.token_info,
            ctx.accounts.pyth_oracle.as_ref(),
            clock.unix_timestamp,
        )?
    } else {
        None
    };

    lending_operations::refresh_reserve(reserve, clock, price_res)?;
    lending_operations::refresh_reserve_limit_timestamps(reserve, clock.slot)?;

    xmsg!(
        "Token: {} Price: {}",
        &reserve.config.token_info.symbol(),
        reserve.liquidity.get_market_price_f()
    );

    Ok(())
}

#[derive(Accounts)]
pub struct RefreshReserveCtx<'info> {
    #[account(mut,
        has_one = lending_market,
    )]
    pub reserve: AccountLoader<'info, Reserve>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    pub pyth_oracle: Option<AccountInfo<'info>>,
}
