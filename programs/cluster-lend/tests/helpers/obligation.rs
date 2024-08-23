use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token;
use anyhow::Result;
use cluster_lend::{
    utils::pda::{init_obligation_pda, init_reserve_pdas_program_id, lending_market_auth},
    InitObligationArgs,
};
use solana_program::{instruction::Instruction, rent::Rent, sysvar::SysvarId};
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{reserve::ReserveFixture, test::TestFixture};

pub struct ObligationFixture {
    pub key: Pubkey,
    pub owner: Pubkey,
    pub payer: Pubkey,
    pub lending_market: Pubkey,
}

impl ObligationFixture {
    pub async fn new(
        ctx: &TestFixture,
        owner: &Keypair,
        lending_market: Pubkey,
        args: InitObligationArgs,
    ) -> ObligationFixture {
        let obligation_key = init_obligation_pda(
            &owner.pubkey(),
            &lending_market,
            &Pubkey::default(),
            &Pubkey::default(),
            &args,
        );

        let accounts = cluster_lend::accounts::InitializeObligationCtx {
            owner: owner.pubkey(),
            fee_payer: ctx.payer(),
            lending_market,
            obligation: obligation_key,
            seed1_account: Pubkey::default(),
            seed2_account: Pubkey::default(),
            rent: Rent::id(),
            token_program: token::ID,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::InitializeObligation { args }.data(),
        };

        ctx.send_transaction(&[ix], &[owner]).await.unwrap();

        ObligationFixture {
            key: obligation_key,
            owner: owner.pubkey(),
            payer: ctx.payer(),
            lending_market,
        }
    }

    pub fn initialize_obligation_ix(&self, args: InitObligationArgs) -> Instruction {
        let accounts = cluster_lend::accounts::InitializeObligationCtx {
            owner: self.owner,
            fee_payer: self.payer,
            lending_market: self.lending_market,
            obligation: self.key,
            seed1_account: Pubkey::default(),
            seed2_account: Pubkey::default(),
            rent: Rent::id(),
            token_program: token::ID,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::InitializeObligation { args }.data(),
        };

        ix
    }

    pub fn refresh_ix(&self, reserve_keys: Vec<Pubkey>) -> Instruction {
        let mut remain_accounts: Vec<AccountMeta> = reserve_keys
            .iter()
            .map(|t| AccountMeta {
                is_signer: false,
                is_writable: true,
                pubkey: *t,
            })
            .collect();

        let accounts = cluster_lend::accounts::RefreshObligationCtx {
            lending_market: self.lending_market,
            obligation: self.key,
        };

        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: [accounts.to_account_metas(Some(true)), remain_accounts].concat(),
            data: cluster_lend::instruction::RefreshObligation {}.data(),
        };

        ix
    }

    pub fn deposit_collateral_ix(
        &self,
        collateral_amount: u64,
        deposit_reserve: Pubkey,
        reserve_destination_collateral: Pubkey,
        user_source_collateral: Pubkey,
    ) -> Instruction {
        let accounts = cluster_lend::accounts::DepositObligationCollateralCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            obligation: self.key,
            deposit_reserve,
            reserve_destination_collateral,
            user_source_collateral,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };

        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::DepositObligationCollateral { collateral_amount }
                .data(),
        };

        ix
    }

    pub fn deposit_liquidity_collateral_ix(
        &self,
        liquidity_amount: u64,
        reserve: &ReserveFixture,
        user_source_liquidity: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &reserve.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::DepositLiquidityCollateralCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            lending_market_authority,
            obligation: self.key,
            reserve: reserve.key,
            reserve_liquidity_supply: pdas.liquidity_supply_vault,
            reserve_collateral_mint: pdas.collateral_ctoken_mint,
            reserve_destination_deposit_collateral: pdas.collateral_supply_vault,
            user_source_liquidity,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };

        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::DepositLiquidityCollateral { liquidity_amount }.data(),
        };

        ix
    }

    pub fn withdraw_collateral_ix(
        &self,
        collateral_amount: u64,
        reserve: &ReserveFixture,
        user_destination_collateral: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &reserve.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::WithdrawObligationCollateralCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            lending_market_authority,
            obligation: self.key,
            withdraw_reserve: reserve.key,
            reserve_source_collateral: pdas.collateral_supply_vault,
            user_destination_collateral,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };

        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::WithdrawObligationCollateral { collateral_amount }
                .data(),
        };

        ix
    }

    pub fn borrow_liquidity_ix(
        &self,
        liquidity_amount: u64,
        reserve: &ReserveFixture,
        user_destination_liquidity: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &reserve.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::BorrowObligationLiquidityCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            lending_market_authority,
            obligation: self.key,
            borrow_reserve: reserve.key,
            reserve_source_liquidity: pdas.liquidity_supply_vault,
            borrow_reserve_liquidity_fee_receiver: pdas.fee_vault,
            user_destination_liquidity,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::BorrowObligationLiquidity { liquidity_amount }.data(),
        };

        ix
    }

    pub fn repay_liquidity_ix(
        &self,
        liquidity_amount: u64,
        reserve: &ReserveFixture,
        user_source_liquidity: Pubkey,
    ) -> Instruction {
        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &reserve.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::RepayObligationLiquidityCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            obligation: self.key,
            repay_reserve: reserve.key,
            reserve_destination_liquidity: pdas.liquidity_supply_vault,
            user_source_liquidity,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::RepayObligationLiquidity { liquidity_amount }.data(),
        };

        ix
    }

    pub fn liquidate_ix(
        &self,
        liquidity_amount: u64,
        max_allowed_ltv_override_percent: u64,
        min_acceptable_received_collateral_amount: u64,
        liquidator: Pubkey,
        repay_reserve: Pubkey,
        repay_reserve_liquidity_supply: Pubkey,
        withdraw_reserve: Pubkey,
        withdraw_liquidity_mint: Pubkey,
        user_destination_collateral: Pubkey,
        user_source_liquidity: Pubkey,
        user_destination_liquidity: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &withdraw_liquidity_mint,
        );

        let accounts = cluster_lend::accounts::LiquidateObligationCtx {
            liquidator,
            lending_market: self.lending_market,
            lending_market_authority,
            obligation: self.key,
            repay_reserve,
            repay_reserve_liquidity_supply,
            withdraw_reserve,
            withdraw_reserve_collateral_mint: pdas.collateral_ctoken_mint,
            withdraw_reserve_collateral_supply: pdas.collateral_supply_vault,
            withdraw_reserve_liquidity_fee_receiver: pdas.fee_vault,
            withdraw_reserve_liquidity_supply: pdas.liquidity_supply_vault,
            user_source_liquidity,
            user_destination_collateral,
            user_destination_liquidity,
            token_program: token::ID,
            instruction_sysvar_account: Instructions::id(),
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::LiquidateObligation {
                liquidity_amount,
                max_allowed_ltv_override_percent,
                min_acceptable_received_collateral_amount,
            }
            .data(),
        };

        ix
    }
}
