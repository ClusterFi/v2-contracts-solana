use anchor_lang::prelude::*;

use crate::{
    constants::VALUE_BYTE_MAX_ARRAY_LEN_MARKET_UPDATE,
    errors::LendingError,
    state::{LendingMarket, UpdateLendingMarketMode},
    utils::{validate_numerical_bool, Fraction}, xmsg,
};

pub fn process_update_market(
    ctx: Context<UpdateMarketCtx>,
    mode: u64,
    value: [u8; VALUE_BYTE_MAX_ARRAY_LEN_MARKET_UPDATE],
) -> Result<()> {
    let mode = UpdateLendingMarketMode::try_from(mode)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let market = &mut ctx.accounts.lending_market.load_mut()?;

    xmsg!(
        "Updating lending market with mode {:?} and value {:?}",
        mode,
        &value[0..32]
    );

    match mode {
        UpdateLendingMarketMode::UpdateEmergencyMode => {
            let emergency_mode = value[0];
            xmsg!("Value is {:?}", emergency_mode);
            if emergency_mode == 0 {
                market.emergency_mode = 0
            } else if emergency_mode == 1 {
                market.emergency_mode = 1;
            } else {
                return err!(LendingError::InvalidFlag);
            }
        }
        UpdateLendingMarketMode::UpdateLiquidationCloseFactor => {
            let liquidation_close_factor = value[0];
            xmsg!("Value is {:?}", liquidation_close_factor);
            if !(5..=100).contains(&liquidation_close_factor) {
                return err!(LendingError::InvalidFlag);
            }
            market.liquidation_max_debt_close_factor_pct = liquidation_close_factor;
        }
        UpdateLendingMarketMode::UpdateLiquidationMaxValue => {
            let value = u64::from_le_bytes(value[..8].try_into().unwrap());
            xmsg!("Value is {:?}", value);
            if value == 0 {
                return err!(LendingError::InvalidFlag);
            }
            market.max_liquidatable_debt_market_value_at_once = value;
        }
        UpdateLendingMarketMode::UpdateGlobalAllowedBorrow => {
            let value = u64::from_le_bytes(value[..8].try_into().unwrap());
            xmsg!("Value is {:?}", value);
            market.global_allowed_borrow_value = value;
        }
        UpdateLendingMarketMode::UpdateGlobalUnhealthyBorrow => {
            let value = u64::from_le_bytes(value[..8].try_into().unwrap());
            xmsg!("Value is {:?}", value);
            market.global_unhealthy_borrow_value = value;
        }
        UpdateLendingMarketMode::UpdateMinFullLiquidationThreshold => {
            let value = u64::from_le_bytes(value[..8].try_into().unwrap());
            xmsg!("Value is {:?}", value);
            if value == 0 {
                return err!(LendingError::InvalidFlag);
            }
            market.min_full_liquidation_value_threshold = value;
        }
        UpdateLendingMarketMode::UpdateInsolvencyRiskLtv => {
            let insolvency_risk_ltv = value[0];
            xmsg!("Value is {:?}", insolvency_risk_ltv);

            if !(5..=100).contains(&insolvency_risk_ltv) {
                return err!(LendingError::InvalidFlag);
            }
            market.insolvency_risk_unhealthy_ltv_pct = insolvency_risk_ltv;
        }
        UpdateLendingMarketMode::UpdatePriceRefreshTriggerToMaxAgePct => {
            let value = value[0];
            xmsg!("Value is {:?}", value);
            if value > 100 {
                xmsg!("Price refresh trigger to max age pct must be in range [0, 100]");
                return err!(LendingError::InvalidConfig);
            }
            market.price_refresh_trigger_to_max_age_pct = value;
        }
        UpdateLendingMarketMode::UpdateAutodeleverageEnabled => {
            let autodeleverage_enabled = value[0];
            xmsg!("Prev Value is {:?}", market.autodeleverage_enabled);
            xmsg!("New Value is {:?}", autodeleverage_enabled);
            if autodeleverage_enabled == 0 {
                market.autodeleverage_enabled = 0
            } else if autodeleverage_enabled == 1 {
                market.autodeleverage_enabled = 1;
            } else {
                xmsg!(
                    "Autodeleverage enabled flag must be 0 or 1, got {:?}",
                    autodeleverage_enabled
                );
                return err!(LendingError::InvalidFlag);
            }
        }
        UpdateLendingMarketMode::UpdateBorrowingDisabled => {
            let borrow_disabled = value[0];
            xmsg!("Prev Value is {:?}", market.borrow_disabled);
            xmsg!("New Value is {:?}", borrow_disabled);
            validate_numerical_bool(borrow_disabled)?;
            market.borrow_disabled = borrow_disabled;
        }
        UpdateLendingMarketMode::UpdateMinNetValueObligationPostAction => {
            let min_net_value_in_obligation_sf =
                u128::from_le_bytes(value[..16].try_into().unwrap());
            xmsg!(
                "Prev Value is {}",
                Fraction::from_bits(market.min_net_value_in_obligation_sf)
            );
            xmsg!(
                "New Value is {}",
                Fraction::from_bits(min_net_value_in_obligation_sf)
            );
            market.min_net_value_in_obligation_sf = min_net_value_in_obligation_sf;
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateMarketCtx<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner
    )]
    pub lending_market: AccountLoader<'info, LendingMarket>,

    pub system_program: Program<'info, System>,
}
