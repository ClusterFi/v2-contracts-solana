use std::{
    cmp::{max, min},
    ops::{Add, Div, Mul},
};

use anchor_lang::{
    account, err,
    prelude::{Pubkey, *},
    solana_program::clock::Slot,
    Result,
};
use borsh::{BorshDeserialize, BorshSerialize};
use derivative::Derivative;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use strum::EnumString;

use super::{LastUpdate, TokenInfo};
use crate::{
    constants::{INITIAL_COLLATERAL_RATE, PROGRAM_VERSION, RESERVE_CONFIG_SIZE, SLOTS_PER_YEAR},
    errors::{LendingError, LendingResult},
    state::{CalculateBorrowResult, CalculateRepayResult},
    utils::{borrow_rate_curve::BorrowRateCurve, BigFraction, Fraction, FractionExtra},
    xmsg,
};

#[derive(Default, Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct BigFractionBytes {
    pub value: [u64; 4],
    pub padding: [u64; 2],
}

impl From<BigFraction> for BigFractionBytes {
    fn from(value: BigFraction) -> BigFractionBytes {
        BigFractionBytes {
            value: value.to_bits(),
            padding: [0; 2],
        }
    }
}

impl From<BigFractionBytes> for BigFraction {
    fn from(value: BigFractionBytes) -> BigFraction {
        BigFraction::from_bits(value.value)
    }
}

// static_assertions::const_assert_eq!(RESERVE_SIZE, std::mem::size_of::<Reserve>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<Reserve>() % 8);
#[derive(PartialEq, Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
#[repr(C)]
pub struct Reserve {
    pub version: u64,

    pub last_update: LastUpdate,

    pub lending_market: Pubkey,

    #[derivative(Debug = "ignore")]
    pub padding: u64,

    pub liquidity: ReserveLiquidity,

    pub collateral: ReserveCollateral,

    pub config: ReserveConfig,

    pub reserved: [u64; 32],
}

impl Default for Reserve {
    fn default() -> Self {
        Self {
            version: 0,
            last_update: LastUpdate::default(),
            lending_market: Pubkey::default(),
            liquidity: ReserveLiquidity::default(),
            collateral: ReserveCollateral::default(),
            config: ReserveConfig::default(),
            padding: 0,
            reserved: [0; 32],
        }
    }
}

impl Reserve {
    pub fn init(&mut self, params: InitReserveParams) {
        *self = Self::default();
        self.version = PROGRAM_VERSION as u64;
        self.last_update = LastUpdate::new(params.current_slot);
        self.lending_market = params.lending_market;
        self.liquidity = *params.liquidity;
        self.collateral = *params.collateral;
        self.config = *params.config;
    }

    pub fn current_borrow_rate(&self) -> Result<Fraction> {
        let utilization_rate = self.liquidity.utilization_rate()?;

        self.config
            .borrow_rate_curve
            .get_borrow_rate(utilization_rate)
    }

    pub fn borrow_factor_f(&self) -> Fraction {
        Fraction::from_percent(self.config.borrow_factor_pct)
    }

    pub fn token_symbol(&self) -> &str {
        self.config.token_info.symbol()
    }

    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> Result<u64> {
        let collateral_amount = self
            .collateral_exchange_rate()?
            .liquidity_to_collateral(liquidity_amount);

        self.liquidity.deposit(liquidity_amount)?;
        self.collateral.mint(collateral_amount)?;

        Ok(collateral_amount)
    }

    pub fn redeem_collateral(&mut self, collateral_amount: u64) -> Result<u64> {
        let collateral_exchange_rate = self.collateral_exchange_rate()?;

        let liquidity_amount =
            collateral_exchange_rate.collateral_to_liquidity(collateral_amount)?;

        self.collateral.burn(collateral_amount)?;
        self.liquidity.withdraw(liquidity_amount)?;

        Ok(liquidity_amount)
    }

    pub fn collateral_exchange_rate(&self) -> LendingResult<CollateralExchangeRate> {
        let total_liquidity = self.liquidity.total_supply()?;
        self.collateral.exchange_rate(total_liquidity)
    }

    pub fn accrue_interest(&mut self, current_slot: Slot) -> Result<()> {
        let slots_elapsed = self.last_update.slots_elapsed(current_slot)?;
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate()?;
            let protocol_take_rate = Fraction::from_percent(self.config.protocol_take_rate_pct);

            self.liquidity.compound_interest(
                current_borrow_rate,
                slots_elapsed,
                protocol_take_rate,
            )?;
        }

        Ok(())
    }

    pub fn update_deposit_limit_crossed_slot(&mut self, current_slot: Slot) -> Result<()> {
        if self.deposit_limit_crossed()? {
            if self.liquidity.deposit_limit_crossed_slot == 0 {
                self.liquidity.deposit_limit_crossed_slot = current_slot;
            }
        } else {
            self.liquidity.deposit_limit_crossed_slot = 0;
        }
        Ok(())
    }

    pub fn update_borrow_limit_crossed_slot(&mut self, current_slot: Slot) -> Result<()> {
        if self.borrow_limit_crossed()? {
            if self.liquidity.borrow_limit_crossed_slot == 0 {
                self.liquidity.borrow_limit_crossed_slot = current_slot;
            }
        } else {
            self.liquidity.borrow_limit_crossed_slot = 0;
        }
        Ok(())
    }

    pub fn calculate_borrow(
        &self,
        amount_to_borrow: u64,
        max_borrow_factor_adjusted_debt_value: Fraction,
        remaining_reserve_borrow: Fraction,
    ) -> Result<CalculateBorrowResult> {
        let decimals = 10u64
            .checked_pow(self.liquidity.mint_decimals as u32)
            .ok_or(LendingError::MathOverflow)?;
        let market_price_f = self.liquidity.get_market_price_f();

        if amount_to_borrow == u64::MAX {
            let borrow_amount_f = (max_borrow_factor_adjusted_debt_value * u128::from(decimals)
                / market_price_f
                / self.borrow_factor_f())
            .min(remaining_reserve_borrow)
            .min(self.liquidity.available_amount.into());
            let borrow_fee = self
                .config
                .fees
                .calculate_borrow_fees(borrow_amount_f, FeeCalculation::Inclusive)?;
            let borrow_amount: u64 = borrow_amount_f.to_floor();
            let receive_amount = borrow_amount - borrow_fee;

            Ok(CalculateBorrowResult {
                borrow_amount_f,
                receive_amount,
                borrow_fee,
            })
        } else {
            let receive_amount = amount_to_borrow;
            let mut borrow_amount_f = Fraction::from(receive_amount);
            let borrow_fee = self
                .config
                .fees
                .calculate_borrow_fees(borrow_amount_f, FeeCalculation::Exclusive)?;

            borrow_amount_f += Fraction::from_num(borrow_fee);
            let borrow_factor_adjusted_debt_value = borrow_amount_f
                .mul(market_price_f)
                .div(u128::from(decimals))
                .mul(self.borrow_factor_f());
            if borrow_factor_adjusted_debt_value > max_borrow_factor_adjusted_debt_value {
                xmsg!("Borrow value cannot exceed maximum borrow value, borrow borrow_factor_adjusted_debt_value: {}, max_borrow_factor_adjusted_debt_value: {}",
                    borrow_factor_adjusted_debt_value, max_borrow_factor_adjusted_debt_value);
                return err!(LendingError::BorrowTooLarge);
            }

            Ok(CalculateBorrowResult {
                borrow_amount_f,
                receive_amount,
                borrow_fee,
            })
        }
    }

    pub fn calculate_repay(
        &self,
        amount_to_repay: u64,
        borrowed_amount: Fraction,
    ) -> LendingResult<CalculateRepayResult> {
        let settle_amount_f = if amount_to_repay == u64::MAX {
            borrowed_amount
        } else {
            let amount_to_repay_f = Fraction::from(amount_to_repay);
            min(amount_to_repay_f, borrowed_amount)
        };
        let repay_amount = settle_amount_f.to_ceil();

        Ok(CalculateRepayResult {
            settle_amount_f,
            repay_amount,
        })
    }

    pub fn calculate_redeem_fees(&self) -> Result<u64> {
        Ok(min(
            self.liquidity.available_amount,
            Fraction::from_bits(self.liquidity.accumulated_protocol_fees_sf).to_floor(),
        ))
    }

    pub fn deposit_limit_crossed(&self) -> Result<bool> {
        let crossed = self.liquidity.total_supply()? > Fraction::from(self.config.deposit_limit);
        Ok(crossed)
    }

    pub fn borrow_limit_crossed(&self) -> Result<bool> {
        let crossed = self.liquidity.total_borrow() > Fraction::from(self.config.borrow_limit);
        Ok(crossed)
    }
}

pub struct InitReserveParams {
    pub current_slot: Slot,
    pub lending_market: Pubkey,
    pub liquidity: Box<ReserveLiquidity>,
    pub collateral: Box<ReserveCollateral>,
    pub config: Box<ReserveConfig>,
}

#[derive(Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,
    pub supply_vault: Pubkey,
    pub fee_vault: Pubkey,
    pub available_amount: u64,
    pub padding: u64,
    pub borrowed_amount_sf: u128,
    pub market_price_sf: u128,
    pub market_price_last_updated_ts: u64,
    pub mint_decimals: u64,

    pub deposit_limit_crossed_slot: u64,
    pub borrow_limit_crossed_slot: u64,

    pub cumulative_borrow_rate_bsf: BigFractionBytes,
    pub accumulated_protocol_fees_sf: u128,

    pub padding2: [u128; 32],
}

impl Default for ReserveLiquidity {
    fn default() -> Self {
        Self {
            mint_pubkey: Pubkey::default(),
            supply_vault: Pubkey::default(),
            fee_vault: Pubkey::default(),
            available_amount: 0,
            borrowed_amount_sf: 0,
            cumulative_borrow_rate_bsf: BigFractionBytes::from(BigFraction::from(Fraction::ONE)),
            accumulated_protocol_fees_sf: 0,
            market_price_sf: 0,
            mint_decimals: 0,
            deposit_limit_crossed_slot: 0,
            borrow_limit_crossed_slot: 0,
            market_price_last_updated_ts: 0,
            padding: 0,
            padding2: [0; 32],
        }
    }
}

impl ReserveLiquidity {
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_decimals: params.mint_decimals as u64,
            supply_vault: params.supply_vault,
            fee_vault: params.fee_vault,
            available_amount: 0,
            borrowed_amount_sf: 0,
            cumulative_borrow_rate_bsf: BigFractionBytes::from(BigFraction::from(Fraction::ONE)),
            accumulated_protocol_fees_sf: 0,
            market_price_sf: params.market_price_sf,
            deposit_limit_crossed_slot: 0,
            borrow_limit_crossed_slot: 0,
            market_price_last_updated_ts: 0,
            padding: 0,
            padding2: [0; 32],
        }
    }

    pub fn total_supply(&self) -> LendingResult<Fraction> {
        Ok(
            Fraction::from(self.available_amount) + Fraction::from_bits(self.borrowed_amount_sf)
                - Fraction::from_bits(self.accumulated_protocol_fees_sf),
        )
    }

    pub fn total_borrow(&self) -> Fraction {
        Fraction::from_bits(self.borrowed_amount_sf)
    }

    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<()> {
        self.available_amount = self
            .available_amount
            .checked_add(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn withdraw(&mut self, liquidity_amount: u64) -> Result<()> {
        if liquidity_amount > self.available_amount {
            xmsg!("Withdraw amount cannot exceed available amount");
            return err!(LendingError::InsufficientLiquidity);
        }
        self.available_amount = self
            .available_amount
            .checked_sub(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn borrow(&mut self, borrow_f: Fraction) -> Result<()> {
        let borrow_amount: u64 = borrow_f.to_floor();

        if borrow_amount > self.available_amount {
            xmsg!("Borrow amount cannot exceed available amount borrow_amount={}, available_amount={}", borrow_amount, self.available_amount);
            return err!(LendingError::InsufficientLiquidity);
        }

        let borrowed_amount_f = Fraction::from_bits(self.borrowed_amount_sf);

        self.available_amount -= borrow_amount;
        self.borrowed_amount_sf = (borrowed_amount_f + borrow_f).to_bits();

        Ok(())
    }

    pub fn repay(&mut self, repay_amount: u64, settle_amount: Fraction) -> LendingResult<()> {
        self.available_amount = self
            .available_amount
            .checked_add(repay_amount)
            .ok_or(LendingError::MathOverflow)?;
        let borrowed_amount_f = Fraction::from_bits(self.borrowed_amount_sf);
        let safe_settle_amount = min(settle_amount, borrowed_amount_f);
        self.borrowed_amount_sf = borrowed_amount_f
            .checked_sub(safe_settle_amount)
            .ok_or_else(|| {
                xmsg!("Borrowed amount cannot be less than settle amount");
                LendingError::MathOverflow
            })?
            .to_bits();

        Ok(())
    }

    pub fn redeem_fees(&mut self, withdraw_amount: u64) -> Result<()> {
        self.available_amount = self
            .available_amount
            .checked_sub(withdraw_amount)
            .ok_or(LendingError::MathOverflow)?;
        let accumulated_protocol_fees_f = Fraction::from_bits(self.accumulated_protocol_fees_sf);
        let withdraw_amount_f = Fraction::from_num(withdraw_amount);
        self.accumulated_protocol_fees_sf = accumulated_protocol_fees_f
            .checked_sub(withdraw_amount_f)
            .ok_or_else(|| {
                xmsg!("Accumulated protocol fees cannot be less than withdraw amount");
                error!(LendingError::MathOverflow)
            })?
            .to_bits();

        Ok(())
    }

    pub fn utilization_rate(&self) -> LendingResult<Fraction> {
        let total_supply = self.total_supply()?;
        if total_supply == Fraction::ZERO {
            return Ok(Fraction::ZERO);
        }
        Ok(Fraction::from_bits(self.borrowed_amount_sf) / total_supply)
    }

    fn compound_interest(
        &mut self,
        current_borrow_rate: Fraction,
        slots_elapsed: u64,
        protocol_take_rate: Fraction,
    ) -> LendingResult<()> {
        let previous_cumulative_borrow_rate = BigFraction::from(self.cumulative_borrow_rate_bsf);
        let previous_debt_f = Fraction::from_bits(self.borrowed_amount_sf);
        let acc_protocol_fees_f = Fraction::from_bits(self.accumulated_protocol_fees_sf);

        let compounded_interest_rate =
            approximate_compounded_interest(current_borrow_rate, slots_elapsed);

        let new_cumulative_borrow_rate: BigFraction =
            previous_cumulative_borrow_rate * BigFraction::from(compounded_interest_rate);

        let new_debt_f = previous_debt_f * compounded_interest_rate;
        let net_new_debt_f = new_debt_f - previous_debt_f;

        let total_protocol_fee_f = net_new_debt_f * protocol_take_rate;

        let new_acc_protocol_fees_f = total_protocol_fee_f + acc_protocol_fees_f;

        self.cumulative_borrow_rate_bsf = new_cumulative_borrow_rate.into();
        self.accumulated_protocol_fees_sf = new_acc_protocol_fees_f.to_bits();
        self.borrowed_amount_sf = new_debt_f.to_bits();

        Ok(())
    }

    pub fn forgive_debt(&mut self, liquidity_amount: Fraction) -> LendingResult<()> {
        let amt = Fraction::from_bits(self.borrowed_amount_sf);
        let new_amt = amt - liquidity_amount;
        self.borrowed_amount_sf = new_amt.to_bits();

        Ok(())
    }

    pub fn get_market_price_f(&self) -> Fraction {
        Fraction::from_bits(self.market_price_sf)
    }
}

pub struct NewReserveLiquidityParams {
    pub mint_pubkey: Pubkey,
    pub mint_decimals: u8,
    pub supply_vault: Pubkey,
    pub fee_vault: Pubkey,
    pub market_price_sf: u128,
}

#[derive(Debug, Default, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct ReserveCollateral {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub supply_vault: Pubkey,
}

impl ReserveCollateral {
    pub fn new(params: NewReserveCollateralParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            supply_vault: params.supply_vault,
        }
    }

    pub fn mint(&mut self, collateral_amount: u64) -> Result<()> {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn burn(&mut self, collateral_amount: u64) -> Result<()> {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_sub(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    fn exchange_rate(&self, total_liquidity: Fraction) -> LendingResult<CollateralExchangeRate> {
        let rate = if self.mint_total_supply == 0 || total_liquidity == Fraction::ZERO {
            INITIAL_COLLATERAL_RATE
        } else {
            Fraction::from(self.mint_total_supply) / total_liquidity
        };

        Ok(CollateralExchangeRate(rate))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollateralExchangeRate(Fraction);

impl CollateralExchangeRate {
    pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> LendingResult<u64> {
        Ok(self
            .fraction_collateral_to_liquidity(collateral_amount.into())
            .to_floor())
    }

    pub fn fraction_collateral_to_liquidity(&self, collateral_amount: Fraction) -> Fraction {
        collateral_amount / self.0
    }

    pub fn liquidity_to_collateral(&self, liquidity_amount: u64) -> u64 {
        (self.0 * u128::from(liquidity_amount)).to_floor()
    }
}

impl From<CollateralExchangeRate> for Fraction {
    fn from(exchange_rate: CollateralExchangeRate) -> Self {
        exchange_rate.0
    }
}

impl From<Fraction> for CollateralExchangeRate {
    fn from(fraction: Fraction) -> Self {
        Self(fraction)
    }
}

pub struct NewReserveCollateralParams {
    pub mint_pubkey: Pubkey,
    pub supply_vault: Pubkey,
}

static_assertions::const_assert_eq!(RESERVE_CONFIG_SIZE, std::mem::size_of::<ReserveConfig>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveConfig>() % 8);
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Derivative, Default)]
#[derivative(Debug)]
#[zero_copy]
#[repr(C)]
pub struct ReserveConfig {
    pub status: u8,
    pub asset_tier: u8,
    pub protocol_take_rate_pct: u8,
    pub protocol_liquidation_fee_pct: u8,
    pub loan_to_value_pct: u8,
    pub liquidation_threshold_pct: u8,
    pub min_liquidation_bonus_bps: u16,
    pub max_liquidation_bonus_bps: u16,
    pub bad_debt_liquidation_bonus_bps: u16,

    #[derivative(Debug = "ignore")]
    pub padding_0: [u8; 4],

    pub deleveraging_margin_call_period_secs: u64,
    pub deleveraging_threshold_slots_per_bps: u64,
    pub fees: ReserveFees,
    pub borrow_rate_curve: BorrowRateCurve,
    pub borrow_factor_pct: u64,

    pub deposit_limit: u64,
    pub borrow_limit: u64,
    pub token_info: TokenInfo,

    pub deposit_withdrawal_cap: WithdrawalCaps,
    pub debt_withdrawal_cap: WithdrawalCaps,
    pub padding_1: u8,

    #[derivative(Debug = "ignore")]
    pub padding_2: [u8; 7],

    pub reserved: [u64; 32],
}

impl ReserveConfig {
    pub fn get_asset_tier(&self) -> AssetTier {
        AssetTier::try_from(self.asset_tier).unwrap()
    }

    pub fn get_borrow_factor(&self) -> Fraction {
        max(
            Fraction::ONE,
            Fraction::from_percent(self.borrow_factor_pct),
        )
    }

    pub fn status(&self) -> ReserveStatus {
        ReserveStatus::try_from(self.status).unwrap()
    }
}

#[repr(u8)]
#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    TryFromPrimitive,
    IntoPrimitive,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Copy,
)]
pub enum ReserveStatus {
    Active = 0,
    Obsolete = 1,
    Hidden = 2,
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Default, Debug)]
#[zero_copy]
#[repr(C)]
pub struct WithdrawalCaps {
    pub config_capacity: i64,
    pub current_total: i64,
    pub last_interval_start_timestamp: u64,
    pub config_interval_length_seconds: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Default, PartialEq, Eq, Derivative)]
#[derivative(Debug)]
#[zero_copy]
#[repr(C)]
pub struct ReserveFees {
    pub borrow_fee_sf: u64,
    pub flash_loan_fee_sf: u64,
    #[derivative(Debug = "ignore")]
    pub padding: [u8; 8],
}

impl ReserveFees {
    pub fn calculate_borrow_fees(
        &self,
        borrow_amount: Fraction,
        fee_calculation: FeeCalculation,
    ) -> Result<u64> {
        self.calculate_fees(borrow_amount, self.borrow_fee_sf, fee_calculation)
    }

    pub fn calculate_flash_loan_fees(&self, flash_loan_amount_f: Fraction) -> Result<u64> {
        let protocol_fee = self.calculate_fees(
            flash_loan_amount_f,
            self.flash_loan_fee_sf,
            FeeCalculation::Exclusive,
        )?;

        Ok(protocol_fee)
    }

    fn calculate_fees(
        &self,
        amount: Fraction,
        fee_sf: u64,
        fee_calculation: FeeCalculation,
    ) -> Result<u64> {
        let borrow_fee_rate = Fraction::from_bits(fee_sf.into());
        if borrow_fee_rate > Fraction::ZERO && amount > Fraction::ZERO {
            let minimum_fee = 1u64;

            let borrow_fee_amount = match fee_calculation {
                FeeCalculation::Exclusive => amount.mul(borrow_fee_rate),
                FeeCalculation::Inclusive => {
                    let borrow_fee_rate = borrow_fee_rate.div(borrow_fee_rate.add(Fraction::ONE));
                    amount.mul(borrow_fee_rate)
                }
            };

            let borrow_fee_f = borrow_fee_amount.max(minimum_fee.into());
            if borrow_fee_f >= amount {
                xmsg!("Borrow amount is too small to receive liquidity after fees");
                return err!(LendingError::BorrowTooSmall);
            }

            let protocol_fee: u64 = borrow_fee_f.to_round();
            Ok(protocol_fee)
        } else {
            Ok(0)
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq)]
pub enum FeeCalculation {
    Exclusive,
    Inclusive,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Debug,
    PartialEq,
    Eq,
    num_enum::IntoPrimitive,
    num_enum::TryFromPrimitive,
)]
#[repr(u8)]
pub enum AssetTier {
    Regular = 0,
    IsolatedCollateral = 1,
    IsolatedDebt = 2,
}

pub fn approximate_compounded_interest(rate: Fraction, elapsed_slots: u64) -> Fraction {
    let base = rate / u128::from(SLOTS_PER_YEAR);
    match elapsed_slots {
        0 => return Fraction::ONE,
        1 => return Fraction::ONE + base,
        2 => return (Fraction::ONE + base) * (Fraction::ONE + base),
        3 => return (Fraction::ONE + base) * (Fraction::ONE + base) * (Fraction::ONE + base),
        4 => {
            let pow_two = (Fraction::ONE + base) * (Fraction::ONE + base);
            return pow_two * pow_two;
        }
        _ => (),
    }

    let exp: u128 = elapsed_slots.into();
    let exp_minus_one = exp.wrapping_sub(1);
    let exp_minus_two = exp.wrapping_sub(2);

    let base_power_two = base * base;
    let base_power_three = base_power_two * base;

    let first_term = base * exp;

    let second_term = (base_power_two * exp * exp_minus_one) / 2;

    let third_term = (base_power_three * exp * exp_minus_one * exp_minus_two) / 6;

    Fraction::ONE + first_term + second_term + third_term
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    TryFromPrimitive,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    EnumString,
)]
#[repr(u64)]
pub enum UpdateConfigMode {
    UpdateLoanToValuePct = 1,
    UpdateMaxLiquidationBonusBps = 2,
    UpdateLiquidationThresholdPct = 3,
    UpdateProtocolLiquidationFee = 4,
    UpdateProtocolTakeRate = 5,
    UpdateFeesBorrowFee = 6,
    UpdateFeesFlashLoanFee = 7,
    UpdateFeesReferralFeeBps = 8,
    UpdateDepositLimit = 9,
    UpdateBorrowLimit = 10,
    UpdateTokenInfoTwapDivergence = 14,
    UpdateTokenInfoName = 17,
    UpdateTokenInfoPriceMaxAge = 18,
    UpdateTokenInfoTwapMaxAge = 19,
    UpdatePythPrice = 21,
    UpdateBorrowRateCurve = 24,
    UpdateEntireReserveConfig = 25,
    UpdateDebtWithdrawalCap = 26,
    UpdateDepositWithdrawalCap = 27,
    UpdateDebtWithdrawalCapCurrentTotal = 28,
    UpdateDepositWithdrawalCapCurrentTotal = 29,
    UpdateBadDebtLiquidationBonusBps = 30,
    UpdateMinLiquidationBonusBps = 31,
    DeleveragingMarginCallPeriod = 32,
    UpdateBorrowFactor = 33,
    UpdateAssetTier = 34,
    DeleveragingThresholdSlotsPerBps = 36,
    UpdateReserveStatus = 39,
}
