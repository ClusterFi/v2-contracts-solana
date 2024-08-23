use anchor_lang::prelude::*;

use crate::{lending_market::lending_operations, state::LendingMarket, xmsg, Reserve, UpdateConfigMode};

pub fn process_update_reserve(
    ctx: Context<UpdateReserveCtx>,
    mode: u64,
    value: &[u8],
) -> Result<()> {
    let mode =
        UpdateConfigMode::try_from(mode).map_err(|_| ProgramError::InvalidInstructionData)?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let name = reserve.config.token_info.symbol();

    xmsg!(
        "Updating reserve {:?} {} config with mode {:?}",
        ctx.accounts.reserve.key(),
        name,
        mode,
    );

    let clock = Clock::get()?;
    lending_operations::refresh_reserve(reserve, &clock, None)?;

    lending_operations::update_reserve_config(reserve, mode, &value);

    lending_operations::utils::validate_reserve_config(&reserve.config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateReserveCtx<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner
    )]
    pub lending_market: AccountLoader<'info, LendingMarket>,

    #[account(mut,
        has_one = lending_market
    )]
    reserve: AccountLoader<'info, Reserve>,
}
