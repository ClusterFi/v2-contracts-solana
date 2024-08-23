use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token;
use anyhow::Result;
use cluster_lend::utils::pda::lending_market_auth;
use solana_program::{instruction::Instruction, sysvar};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use std::{cell::RefCell, mem, rc::Rc};

pub struct LendingMarketFixture {
    pub key: Pubkey,
    pub owner: Pubkey,
}

impl LendingMarketFixture {
    pub fn init_market_ix(&self, quote_currency: [u8; 32]) -> Instruction {
        let lending_market_authority = lending_market_auth(&self.key);

        let accounts = cluster_lend::accounts::InitializeMarketCtx {
            owner: self.owner,
            lending_market: self.key,
            lending_market_authority,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::InitializeMarket { quote_currency }.data(),
        };

        ix
    }

    pub fn update_market_ix(&self, mode: u64, value: [u8; 72]) -> Instruction {
        let accounts = cluster_lend::accounts::UpdateMarketCtx {
            owner: self.owner,
            lending_market: self.key,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::UpdateMarket { mode, value }.data(),
        };

        ix
    }

    pub fn update_owner_ix(&self, new_owner: Pubkey) -> Instruction {
        let accounts = cluster_lend::accounts::UpdateMarketOwnerCtx {
            owner: self.owner,
            lending_market: self.key,
            new_owner,
            system_program: system_program::ID,
        };
        let ix = Instruction {
            program_id: cluster_lend::id(),
            accounts: accounts.to_account_metas(Some(true)),
            data: cluster_lend::instruction::UpdateMarketOwner {}.data(),
        };

        ix
    }
}
