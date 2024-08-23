use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token::{self, Token};
use anyhow::Result;
use cluster_lend::{
    constants::VALUE_BYTE_ARRAY_LEN_RESERVE,
    utils::pda::{init_reserve_pdas_program_id, lending_market_auth},
    ReserveConfig,
};
use solana_program::instruction::Instruction;
use solana_sdk::{
    rent,
    sysvar::{instructions, SysvarId},
};

pub struct ReserveFixture {
    pub key: Pubkey,
    pub owner: Pubkey,
    pub payer: Pubkey,
    pub lending_market: Pubkey,
    pub liquidity_mint: Pubkey,
}

impl ReserveFixture {
    pub fn initialize_reserve_ix(&self) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);
        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &self.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::InitializeReserveCtx {
            owner: self.owner,
            lending_market: self.lending_market,
            lending_market_authority,
            reserve: self.key,
            reserve_liquidity_mint: self.liquidity_mint,
            reserve_collateral_mint: pdas.collateral_ctoken_mint,
            reserve_liquidity_supply: pdas.liquidity_supply_vault,
            reserve_collateral_supply: pdas.collateral_supply_vault,
            fee_receiver: pdas.fee_vault,
            rent: rent::Rent::id(),
            token_program: token::ID,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::InitializeReserve {}.data(),
        };

        ix
    }

    pub fn update_reserve_ix(&self, config: ReserveConfig) -> Instruction {
        let mut value = [0; VALUE_BYTE_ARRAY_LEN_RESERVE];
        let data = borsh::BorshSerialize::try_to_vec(&config).unwrap();
        value.copy_from_slice(data.as_slice());

        let accounts = cluster_lend::accounts::UpdateReserveCtx {
            reserve: self.key,
            lending_market: self.lending_market,
            owner: self.owner,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::UpdateReserve { value }.data(),
        };

        ix
    }

    pub fn update_reserve_mode_ix(&self, mode: u64, value: [u8; 32]) -> Instruction {
        let accounts = cluster_lend::accounts::UpdateReserveCtx {
            reserve: self.key,
            lending_market: self.lending_market,
            owner: self.owner,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::UpdateReserveMode { mode, value }.data(),
        };

        ix
    }

    pub fn refresh_ix(&self, pyth_oracle: Option<Pubkey>) -> Instruction {
        let accounts = cluster_lend::accounts::RefreshReserveCtx {
            reserve: self.key,
            lending_market: self.lending_market,
            pyth_oracle,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::RefreshReserve {}.data(),
        };

        ix
    }

    pub fn deposit_liquidity_ix(
        &self,
        liquidity_amount: u64,
        user_source_liquidity: Pubkey,
        user_destination_collateral: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &self.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::DepositReserveLiquidityCtx {
            reserve: self.key,
            lending_market: self.lending_market,
            owner: self.owner,
            lending_market_authority,
            reserve_collateral_mint: pdas.collateral_ctoken_mint,
            reserve_liquidity_supply: pdas.liquidity_supply_vault,
            user_source_liquidity,
            user_destination_collateral,
            token_program: Token::id(),
            instruction_sysvar_account: instructions::id(),
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::DepositReserveLiquidity { liquidity_amount }.data(),
        };

        ix
    }

    pub fn flash_borrow_ix(
        &self,
        liquidity_amount: u64,
        user_transfer_authority: Pubkey,
        user_destination_liquidity: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &self.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::FlashBorrowReserveCtx {
            user_transfer_authority,
            reserve: self.key,
            lending_market: self.lending_market,
            lending_market_authority,
            reserve_source_liquidity: pdas.liquidity_supply_vault,
            reserve_liquidity_fee_receiver: pdas.fee_vault,
            user_destination_liquidity,
            sysvar_info: Instructions::id(),
            token_program: token::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::FlashBorrowReserveLiquidity { liquidity_amount }
                .data(),
        };

        ix
    }

    pub fn flash_repay_ix(
        &self,
        liquidity_amount: u64,
        borrow_instruction_index: u8,
        user_transfer_authority: Pubkey,
        user_source_liquidity: Pubkey,
    ) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.lending_market);

        let pdas = init_reserve_pdas_program_id(
            &cluster_lend::ID,
            &self.lending_market,
            &self.liquidity_mint,
        );

        let accounts = cluster_lend::accounts::FlashRepayReserveCtx {
            user_transfer_authority,
            reserve: self.key,
            lending_market: self.lending_market,
            lending_market_authority,
            reserve_destination_liquidity: pdas.liquidity_supply_vault,
            reserve_liquidity_fee_receiver: pdas.fee_vault,
            user_source_liquidity,
            sysvar_info: Instructions::id(),
            token_program: token::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::FlashRepayReserveLiquidity {
                liquidity_amount,
                borrow_instruction_index,
            }
            .data(),
        };

        ix
    }
}
