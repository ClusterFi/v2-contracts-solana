use anchor_lang::{context::Context, err, prelude::AccountLoader, Bumps, Result};

use crate::{errors::LendingError, state::LendingMarket};

pub fn emergency_mode_disabled(lending_market: &AccountLoader<LendingMarket>) -> Result<()> {
    if lending_market.load()?.emergency_mode > 0 {
        return err!(LendingError::GlobalEmergencyMode);
    }
    Ok(())
}

pub fn check_remaining_accounts<T>(ctx: &Context<T>) -> Result<()>
where
    T: Bumps,
{
    if !ctx.remaining_accounts.is_empty() {
        return err!(LendingError::InvalidAccountInput);
    }

    Ok(())
}
