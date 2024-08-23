use anchor_lang::{prelude::*, solana_program::sysvar, Accounts};
use anchor_spl::token::{self, Token, TokenAccount};

use crate::{
    lending_market::{flash_ixs, lending_checks, lending_operations},
    state::{LendingMarket, Reserve},
    utils::{seeds, token_transfer},
    LendingAction,
};

pub fn process_flash_repay_reserve(
    ctx: Context<FlashRepayReserveCtx>,
    liquidity_amount: u64,
    borrow_instruction_index: u8,
) -> Result<()> {
    lending_checks::flash_repay_reserve_liquidity_checks(&ctx)?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;

    let initial_reserve_token_balance =
        token::accessor::amount(&ctx.accounts.reserve_destination_liquidity.to_account_info())?;
    let initial_reserve_available_liquidity = reserve.liquidity.available_amount;

    flash_ixs::flash_repay_checks(&ctx, borrow_instruction_index, liquidity_amount)?;

    let (flash_loan_amount, reserve_origination_fee) =
        lending_operations::flash_repay_reserve_liquidity(
            reserve,
            liquidity_amount,
            Clock::get()?.slot,
        )?;

    token_transfer::repay_obligation_liquidity_transfer(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.user_source_liquidity.to_account_info(),
        ctx.accounts.reserve_destination_liquidity.to_account_info(),
        ctx.accounts.user_transfer_authority.to_account_info(),
        flash_loan_amount,
    )?;

    if reserve_origination_fee > 0 {
        token_transfer::pay_borrowing_fees_transfer(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.user_source_liquidity.to_account_info(),
            ctx.accounts
                .reserve_liquidity_fee_receiver
                .to_account_info(),
            ctx.accounts.user_transfer_authority.to_account_info(),
            reserve_origination_fee,
        )?;
    }

    lending_checks::post_transfer_vault_balance_liquidity_reserve_checks(
        token::accessor::amount(&ctx.accounts.reserve_destination_liquidity.to_account_info())
            .unwrap(),
        reserve.liquidity.available_amount,
        initial_reserve_token_balance,
        initial_reserve_available_liquidity,
        LendingAction::Additive(flash_loan_amount),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct FlashRepayReserveCtx<'info> {
    pub user_transfer_authority: Signer<'info>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    #[account(mut,
        has_one = lending_market
    )]
    pub reserve: AccountLoader<'info, Reserve>,

    #[account(mut,
        address = reserve.load()?.liquidity.supply_vault
    )]
    pub reserve_destination_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub user_source_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        address = reserve.load()?.liquidity.fee_vault
    )]
    pub reserve_liquidity_fee_receiver: Box<Account<'info, TokenAccount>>,

    /// CHECK: instruction_sysvar account
    #[account(address = sysvar::instructions::ID)]
    pub sysvar_info: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
