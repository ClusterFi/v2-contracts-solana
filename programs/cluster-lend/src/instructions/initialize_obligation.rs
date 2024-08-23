use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::{
    constants::OBLIGATION_SIZE,
    errors::LendingError,
    state::{
        check_obligation_seeds, InitObligationArgs, LendingMarket, Obligation,
        ObligationCollateral, ObligationLiquidity,
    },
};

pub fn process_initialize_obligation(
    ctx: Context<InitializeObligationCtx>,
    args: InitObligationArgs,
) -> Result<()> {
    let clock = &Clock::get()?;

    require!(args.id == 0, LendingError::InvalidObligationId);

    check_obligation_seeds(
        args.tag,
        &ctx.accounts.seed1_account,
        &ctx.accounts.seed2_account,
    )
    .unwrap();

    let obligation = &mut ctx.accounts.obligation.load_init()?;

    obligation.init(crate::state::obligation::InitObligationParams {
        current_slot: clock.slot,
        lending_market: ctx.accounts.lending_market.key(),
        owner: ctx.accounts.owner.key(),
        deposits: [ObligationCollateral::default(); 8],
        borrows: [ObligationLiquidity::default(); 5],
        tag: args.tag as u64,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(args: InitObligationArgs)]
pub struct InitializeObligationCtx<'info> {
    pub owner: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(init,
        seeds = [&[args.tag], &[args.id], owner.key().as_ref(), lending_market.key().as_ref(), seed1_account.key().as_ref(), seed2_account.key().as_ref()],
        bump,
        payer = fee_payer,
        space = OBLIGATION_SIZE + 8,
    )]
    pub obligation: AccountLoader<'info, Obligation>,

    pub lending_market: AccountLoader<'info, LendingMarket>,

    /// CHECK: seed1 account for obligation
    pub seed1_account: AccountInfo<'info>,

    /// CHECK: seed2 account for obligation
    pub seed2_account: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
