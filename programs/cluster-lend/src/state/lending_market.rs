use anchor_lang::prelude::*;
use derivative::Derivative;
use num_enum::TryFromPrimitive;
use strum::EnumString;

use crate::constants::*;

// static_assertions::const_assert_eq!(LENDING_MARKET_SIZE, std::mem::size_of::<LendingMarket>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<LendingMarket>() % 8);
#[derive(PartialEq, Eq, Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
#[repr(C)]
pub struct LendingMarket {
    pub version: u64,
    pub bump: u64,

    pub owner: Pubkey,

    pub quote_currency: [u8; 32],

    pub referral_fee_bps: u16,

    pub emergency_mode: u8,
    pub autodeleverage_enabled: u8,
    pub borrow_disabled: u8,

    pub price_refresh_trigger_to_max_age_pct: u8,
    pub liquidation_max_debt_close_factor_pct: u8,
    pub insolvency_risk_unhealthy_ltv_pct: u8,
    pub min_full_liquidation_value_threshold: u64,

    pub max_liquidatable_debt_market_value_at_once: u64,
    pub global_unhealthy_borrow_value: u64,
    pub global_allowed_borrow_value: u64,

    #[derivative(Debug = "ignore")]
    pub padding: u64,

    pub min_net_value_in_obligation_sf: u128,

    #[derivative(Debug = "ignore")]
    pub reserved: [u64; 32],
}

impl Default for LendingMarket {
    fn default() -> Self {
        Self {
            version: 0,
            bump: 0,
            owner: Pubkey::default(),
            quote_currency: [0; 32],
            emergency_mode: 0,
            autodeleverage_enabled: 0,
            liquidation_max_debt_close_factor_pct: LIQUIDATION_CLOSE_FACTOR,
            insolvency_risk_unhealthy_ltv_pct: CLOSE_TO_INSOLVENCY_RISKY_LTV,
            max_liquidatable_debt_market_value_at_once: MAX_LIQUIDATABLE_VALUE_AT_ONCE,
            global_allowed_borrow_value: GLOBAL_ALLOWED_BORROW_VALUE,
            global_unhealthy_borrow_value: GLOBAL_UNHEALTHY_BORROW_VALUE,
            min_full_liquidation_value_threshold: LIQUIDATION_CLOSE_VALUE,
            referral_fee_bps: 0,
            price_refresh_trigger_to_max_age_pct: 0,
            borrow_disabled: 0,
            min_net_value_in_obligation_sf: MIN_NET_VALUE_IN_OBLIGATION.to_bits(),
            padding: 0,
            reserved: [0; 32],
        }
    }
}

impl LendingMarket {
    pub fn init(&mut self, params: InitLendingMarketParams) {
        *self = Self::default();
        self.version = PROGRAM_VERSION as u64;
        self.owner = params.owner;
        self.bump = params.bump as u64;
        self.quote_currency = params.quote_currency;
    }

    pub fn is_borrowing_disabled(&self) -> bool {
        self.borrow_disabled != false as u8
    }
}

pub struct InitLendingMarketParams {
    pub bump: u8,
    pub owner: Pubkey,
    pub quote_currency: [u8; 32],
}

#[derive(
    TryFromPrimitive,
    AnchorSerialize,
    AnchorDeserialize,
    EnumString,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
)]
#[repr(u64)]
pub enum UpdateLendingMarketMode {
    UpdateEmergencyMode = 1,
    UpdateLiquidationCloseFactor = 2,
    UpdateLiquidationMaxValue = 3,
    UpdateGlobalUnhealthyBorrow = 4,
    UpdateGlobalAllowedBorrow = 5,
    UpdateMinFullLiquidationThreshold = 7,
    UpdateInsolvencyRiskLtv = 8,
    UpdatePriceRefreshTriggerToMaxAgePct = 12,
    UpdateAutodeleverageEnabled = 13,
    UpdateBorrowingDisabled = 14,
    UpdateMinNetValueObligationPostAction = 15,
}
