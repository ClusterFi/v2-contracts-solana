use std::fmt::Formatter;

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{constants::NULL_PUBKEY, errors::LendingError};

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Default)]
#[zero_copy]
#[repr(C)]
pub struct TokenInfo {
    pub name: [u8; 32],

    pub max_twap_divergence_bps: u64,
    pub max_age_price_seconds: u64,
    pub max_age_twap_seconds: u64,
    pub pyth_configuration: PythConfiguration,

    pub _padding: [u64; 20],
}

impl std::fmt::Debug for TokenInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = std::str::from_utf8(&self.name).unwrap_or("InvalidTokenName");
        f.debug_struct("TokenInfo")
            .field("name", &name)
            .field("max_twap_divergence_bps", &self.max_twap_divergence_bps)
            .field("max_age_price_seconds", &self.max_age_price_seconds)
            .field("max_age_twap_seconds", &self.max_age_twap_seconds)
            .field("pyth_configuration", &self.pyth_configuration)
            .finish()
    }
}

impl TokenInfo {
    pub fn validate_token_info_config(&self, pyth_info: &Option<AccountInfo>) -> Result<()> {
        require!(self.is_valid(), LendingError::InvalidOracleConfig);
        require!(self.is_twap_config_valid(), LendingError::InvalidTwapConfig);
        require!(
            self.check_pyth_acc_matches(pyth_info),
            LendingError::InvalidPythPriceAccount
        );
        Ok(())
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.pyth_configuration.is_enabled()
    }

    #[inline]
    pub fn is_twap_enabled(&self) -> bool {
        self.max_twap_divergence_bps > 0
    }

    #[inline]
    pub fn is_twap_config_valid(&self) -> bool {
        if !self.is_twap_enabled() {
            return true;
        }

        if self.max_age_twap_seconds == 0 {
            return false;
        }

        true
    }

    #[inline]
    pub fn check_pyth_acc_matches(&self, pyth_info: &Option<AccountInfo>) -> bool {
        if self.pyth_configuration.is_enabled() {
            matches!(pyth_info, Some(a) if *a.key == self.pyth_configuration.price)
        } else {
            pyth_info.is_none()
        }
    }

    pub fn symbol(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap_or("InvalidTokenName")
            .trim_end_matches('\0')
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq, Default)]
#[zero_copy]
#[repr(transparent)]
pub struct PythConfiguration {
    pub price: Pubkey,
}

impl PythConfiguration {
    pub fn is_enabled(&self) -> bool {
        self.price != Pubkey::default() && self.price != NULL_PUBKEY
    }
}
