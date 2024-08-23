use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    gen_signer_seeds,
    state::{LendingMarket, Reserve},
    utils::{seeds, token_transfer}, xmsg,
};

pub fn process_withdraw_protocol_fees(
    ctx: Context<WithdrawProtocolFeesCtx>,
    withdraw_amount: u64,
) -> Result<()> {
    let market = ctx.accounts.lending_market.load()?;
    let lending_market_key = ctx.accounts.lending_market.key();

    let amount = withdraw_amount.min(ctx.accounts.fee_vault.amount);

    let authority_signer_seeds = gen_signer_seeds!(lending_market_key, market.bump as u8);

    xmsg!("Withdrawing fees: {}", amount);

    token_transfer::withdraw_fees_from_reserve(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.fee_vault.to_account_info(),
        ctx.accounts.lending_market_owner_ata.to_account_info(),
        ctx.accounts.lending_market_authority.to_account_info(),
        authority_signer_seeds,
        amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawProtocolFeesCtx<'info> {
    pub owner: Signer<'info>,

    #[account(has_one = owner)]
    pub lending_market: AccountLoader<'info, LendingMarket>,

    #[account(
        has_one = lending_market
    )]
    pub reserve: AccountLoader<'info, Reserve>,

    /// CHECK: market authority PDA
    #[account(
        seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
        bump = lending_market.load()?.bump as u8,
    )]
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut,
        address = reserve.load()?.liquidity.fee_vault,
        token::authority = lending_market_authority,
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    #[account(mut,
        token::mint = reserve.load()?.liquidity.mint_pubkey,
    )]
    pub lending_market_owner_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
