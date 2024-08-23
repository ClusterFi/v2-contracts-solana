pub mod checks;
pub mod pyth;
pub mod types;
pub mod utils;

use anchor_lang::{prelude::*, solana_program::clock};
use types::TimestampedPrice;

use self::{
    checks::get_validated_price, pyth::get_pyth_price_and_twap, types::TimestampedPriceWithTwap,
};
use crate::{
    errors::LendingError,
    state::{PriceStatusFlags, TokenInfo},
    utils::Fraction, xmsg,
};

const MAX_CONFIDENCE_PERCENTAGE: u64 = 2u64;

const CONFIDENCE_FACTOR: u64 = 100 / MAX_CONFIDENCE_PERCENTAGE;

#[derive(Debug, Clone)]
pub struct GetPriceResult {
    pub price: Fraction,
    pub timestamp: u64,
    pub status: PriceStatusFlags,
}

pub fn get_price(
    token_info: &TokenInfo,
    pyth_price_account_info: Option<&AccountInfo>,
    unix_timestamp: clock::UnixTimestamp,
) -> Result<Option<GetPriceResult>> {
    let price = get_most_recent_price_and_twap(token_info, pyth_price_account_info)?;

    Ok(get_validated_price(price, token_info, unix_timestamp))
}

fn get_most_recent_price_and_twap(
    token_info: &TokenInfo,
    pyth_price_account_info: Option<&AccountInfo>,
) -> Result<TimestampedPriceWithTwap> {
    let pyth_price = if token_info.pyth_configuration.is_enabled() {
        pyth_price_account_info.and_then(|a| get_pyth_price_and_twap(a).ok())
    } else {
        None
    };

    let most_recent_price = pyth_price;

    most_recent_price.ok_or_else(|| {
        xmsg!("No price feed available");
        error!(LendingError::PriceNotValid)
    })
}
