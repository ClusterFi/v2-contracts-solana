pub mod constants;
pub mod errors;
pub mod instructions;
pub mod lending_market;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;
use constants::{VALUE_BYTE_ARRAY_LEN_RESERVE, VALUE_BYTE_MAX_ARRAY_LEN_MARKET_UPDATE};
use instructions::*;
pub use state::*;
use utils::constraints::emergency_mode_disabled;

declare_id!("FtQFCy8pGnywDh1r2wZJWH8e5KHrkJvDzjTGv3LAAWmj");

#[program]
pub mod cluster_lend {

    use super::*;

    // Market instructions
    pub fn initialize_market(
        ctx: Context<InitializeMarketCtx>,
        quote_currency: [u8; 32],
    ) -> Result<()> {
        process_initialize_market(ctx, quote_currency)
    }

    pub fn update_market(
        ctx: Context<UpdateMarketCtx>,
        mode: u64,
        value: [u8; VALUE_BYTE_MAX_ARRAY_LEN_MARKET_UPDATE],
    ) -> Result<()> {
        process_update_market(ctx, mode, value)
    }

    pub fn update_market_owner(ctx: Context<UpdateMarketOwnerCtx>) -> Result<()> {
        process_update_market_owner(ctx)
    }

    pub fn redeem_fees(ctx: Context<RedeemFeesCtx>) -> Result<()> {
        process_redeem_fees(ctx)
    }

    pub fn withdraw_protocol_fees(
        ctx: Context<WithdrawProtocolFeesCtx>,
        amount: u64,
    ) -> Result<()> {
        process_withdraw_protocol_fees(ctx, amount)
    }

    // Reserve instructions
    pub fn initialize_reserve(ctx: Context<InitializeReserveCtx>) -> Result<()> {
        process_initialize_reserve(ctx)
    }

    pub fn update_reserve(
        ctx: Context<UpdateReserveCtx>,
        value: [u8; VALUE_BYTE_ARRAY_LEN_RESERVE],
    ) -> Result<()> {
        process_update_reserve(ctx, 25, &value)
    }

    pub fn update_reserve_mode(
        ctx: Context<UpdateReserveCtx>,
        mode: u64,
        value: [u8; 32],
    ) -> Result<()> {
        process_update_reserve(ctx, mode, &value)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn refresh_reserve(ctx: Context<RefreshReserveCtx>) -> Result<()> {
        process_refresh_reserve(ctx)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn deposit_reserve_liquidity(
        ctx: Context<DepositReserveLiquidityCtx>,
        liquidity_amount: u64,
    ) -> Result<()> {
        process_deposit_reserve_liquidity(ctx, liquidity_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn redeem_reserve_collateral(
        ctx: Context<RedeemReserveCollateralCtx>,
        collateral_amount: u64,
    ) -> Result<()> {
        process_redeem_reserve_collateral(ctx, collateral_amount)
    }

    // Obligation instructions
    pub fn initialize_obligation(
        ctx: Context<InitializeObligationCtx>,
        args: InitObligationArgs,
    ) -> Result<()> {
        process_initialize_obligation(ctx, args)
    }

    pub fn refresh_obligation(ctx: Context<RefreshObligationCtx>) -> Result<()> {
        process_refresh_obligation(ctx)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn deposit_obligation_collateral(
        ctx: Context<DepositObligationCollateralCtx>,
        collateral_amount: u64,
    ) -> Result<()> {
        process_deposit_obligation_collateral(ctx, collateral_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn deposit_liquidity_collateral(
        ctx: Context<DepositLiquidityCollateralCtx>,
        liquidity_amount: u64,
    ) -> Result<()> {
        process_deposit_liquidity_collateral(ctx, liquidity_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn withdraw_obligation_collateral(
        ctx: Context<WithdrawObligationCollateralCtx>,
        collateral_amount: u64,
    ) -> Result<()> {
        process_withdraw_obligation_collateral(ctx, collateral_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn borrow_obligation_liquidity(
        ctx: Context<BorrowObligationLiquidityCtx>,
        liquidity_amount: u64,
    ) -> Result<()> {
        process_borrow_obligation_liquidity(ctx, liquidity_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn repay_obligation_liquidity(
        ctx: Context<RepayObligationLiquidityCtx>,
        liquidity_amount: u64,
    ) -> Result<()> {
        process_repay_obligation_liquidity(ctx, liquidity_amount)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn liquidate_obligation(
        ctx: Context<LiquidateObligationCtx>,
        liquidity_amount: u64,
        min_acceptable_received_collateral_amount: u64,
        max_allowed_ltv_override_percent: u64,
    ) -> Result<()> {
        process_liquidate_obligation(
            ctx,
            liquidity_amount,
            min_acceptable_received_collateral_amount,
            max_allowed_ltv_override_percent,
        )
    }

    // Flash Loan
    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn flash_repay_reserve_liquidity(
        ctx: Context<FlashRepayReserveCtx>,
        liquidity_amount: u64,
        borrow_instruction_index: u8,
    ) -> Result<()> {
        process_flash_repay_reserve(ctx, liquidity_amount, borrow_instruction_index)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.lending_market))]
    pub fn flash_borrow_reserve_liquidity(
        ctx: Context<FlashBorrowReserveCtx>,
        liquidity_amount: u64,
    ) -> Result<()> {
        process_flash_borrow_reserve(ctx, liquidity_amount)
    }
}
