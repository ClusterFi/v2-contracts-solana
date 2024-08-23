use std::{cell::RefCell, rc::Rc};

use anchor_lang::{accounts::signer, prelude::*};
use anyhow::Result;

use bincode::deserialize;
use cluster_lend::{
    utils::{
        pda::{init_obligation_pda, init_reserve_pdas},
        BorrowRateCurve, CurvePoint,
    },
    AssetTier, InitObligationArgs, PythConfiguration, ReserveConfig, ReserveFees, ReserveStatus,
    TokenInfo, WithdrawalCaps,
};
use pyth_sdk_solana::state::SolanaPriceAccount;
use solana_program::{hash::Hash, sysvar};
use solana_program_test::*;
use solana_sdk::{
    account::AccountSharedData, instruction::Instruction, pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

use crate::{
    lending_market::LendingMarketFixture,
    obligation::ObligationFixture,
    reserve::ReserveFixture,
    spl::MintFixture,
    utils::{clone_keypair, create_pyth_price_account},
};

pub const USDC_MINT_DECIMALS: u8 = 6;
pub const SOL_MINT_DECIMALS: u8 = 9;

pub const USDC_QUOTE_CURRENCY: [u8; 32] =
    *b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
pub const SOL_QUOTE_CURRENCY: [u8; 32] =
    *b"SOL\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

pub const PYTH_USDC_FEED: Pubkey = pubkey!("PythUsdcPrice111111111111111111111111111111");
pub const PYTH_SOL_FEED: Pubkey = pubkey!("PythSo1Price1111111111111111111111111111111");
pub const PYTH_SOL_EQUIVALENT_FEED: Pubkey = pubkey!("PythSo1Equiva1entPrice111111111111111111111");
pub const PYTH_MNDE_FEED: Pubkey = pubkey!("PythMndePrice111111111111111111111111111111");
pub const FAKE_PYTH_USDC_FEED: Pubkey = pubkey!("FakePythUsdcPrice11111111111111111111111111");

pub const TEST_RESERVE_CONFIG: ReserveConfig = ReserveConfig {
    status: 0,     // Active
    asset_tier: 0, // Regular
    protocol_take_rate_pct: 0,
    protocol_liquidation_fee_pct: 0,
    loan_to_value_pct: 75,
    liquidation_threshold_pct: 85,
    min_liquidation_bonus_bps: 200,
    max_liquidation_bonus_bps: 500,
    bad_debt_liquidation_bonus_bps: 10,

    deleveraging_margin_call_period_secs: 259200, // 3 days
    deleveraging_threshold_slots_per_bps: 7200,   // 0.01% per hour
    fees: ReserveFees {
        borrow_fee_sf: 0,
        flash_loan_fee_sf: 0,
        padding: [0; 8],
    },
    borrow_rate_curve: BorrowRateCurve {
        points: [
            CurvePoint {
                utilization_rate_bps: 0,
                borrow_rate_bps: 1,
            },
            CurvePoint {
                utilization_rate_bps: 100,
                borrow_rate_bps: 100,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
            CurvePoint {
                utilization_rate_bps: 10000,
                borrow_rate_bps: 100000,
            },
        ],
    },
    borrow_factor_pct: 100,

    deposit_limit: 10_000_000_000_000,
    borrow_limit: 10_000_000_000_000,

    token_info: TokenInfo {
        name: [0; 32],

        max_twap_divergence_bps: 0,
        max_age_price_seconds: 1_000_000_000,
        max_age_twap_seconds: 0,
        pyth_configuration: PythConfiguration {
            price: PYTH_USDC_FEED,
        },

        _padding: [0; 20],
    },

    deposit_withdrawal_cap: WithdrawalCaps {
        config_capacity: 0,
        current_total: 0,
        last_interval_start_timestamp: 0,
        config_interval_length_seconds: 0,
    },
    debt_withdrawal_cap: WithdrawalCaps {
        config_capacity: 0,
        current_total: 0,
        last_interval_start_timestamp: 0,
        config_interval_length_seconds: 0,
    },

    padding_0: [0; 4],
    padding_1: 0,
    padding_2: [0; 7],
    reserved: [0; 32],
};

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub authority: Keypair,
    pub usdc_mint: MintFixture,
    pub sol_mint: MintFixture,
}

impl TestFixture {
    pub async fn new() -> TestFixture {
        pub fn fixed_entry(
            program_id: &Pubkey,
            accounts: &[anchor_lang::prelude::AccountInfo],
            data: &[u8],
        ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
            let extended_lifetime_accs = unsafe {
                core::mem::transmute::<_, &[anchor_lang::prelude::AccountInfo<'_>]>(accounts)
            };
            cluster_lend::entry(program_id, extended_lifetime_accs, data)
        }

        let mut program =
            ProgramTest::new("cluster_lend", cluster_lend::id(), processor!(fixed_entry));

        let usdc_keypair = Keypair::new();
        let sol_keypair = Keypair::new();
        let authority = Keypair::new();

        program.add_account(
            PYTH_USDC_FEED,
            create_pyth_price_account(usdc_keypair.pubkey(), 1, USDC_MINT_DECIMALS.into(), None),
        );

        program.add_account(
            PYTH_SOL_FEED,
            create_pyth_price_account(usdc_keypair.pubkey(), 1, USDC_MINT_DECIMALS.into(), None),
        );

        let context = Rc::new(RefCell::new(program.start_with_context().await));
        let usdc_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(usdc_keypair),
            Some(USDC_MINT_DECIMALS),
        )
        .await;
        let sol_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(sol_keypair),
            Some(SOL_MINT_DECIMALS),
        )
        .await;

        TestFixture {
            context: Rc::clone(&context),
            usdc_mint: usdc_mint_f,
            sol_mint: sol_mint_f,
            authority,
        }
    }

    pub async fn setup(
        &self,
        user: &Keypair,
        liquidity_mint: &Pubkey,
    ) -> (LendingMarketFixture, ReserveFixture, ObligationFixture) {
        let payer = self.payer_keypair();

        let lending_market_key = Keypair::new();
        let lending_market_f = LendingMarketFixture {
            key: lending_market_key.pubkey(),
            owner: payer.pubkey(),
        };

        let reserve_key = Keypair::new();
        let reserve_f = ReserveFixture {
            key: reserve_key.pubkey(),
            owner: payer.pubkey(),
            payer: payer.pubkey(),
            lending_market: lending_market_f.key,
            liquidity_mint: liquidity_mint.clone(),
        };

        let init_obligation_args = InitObligationArgs { tag: 0, id: 0 };
        let obligation_key = init_obligation_pda(
            &user.pubkey(),
            &lending_market_f.key,
            &Pubkey::default(),
            &Pubkey::default(),
            &init_obligation_args,
        );
        let obligation_f = ObligationFixture {
            key: obligation_key,
            owner: user.pubkey(),
            payer: payer.pubkey(),
            lending_market: lending_market_f.key,
        };

        let _ = self
            .send_transaction(
                &[
                    lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                    reserve_f.initialize_reserve_ix(),
                    reserve_f.update_reserve_ix(TEST_RESERVE_CONFIG),
                    reserve_f.refresh_ix(Some(PYTH_USDC_FEED)),
                    obligation_f.initialize_obligation_ix(init_obligation_args),
                    obligation_f.refresh_ix(vec![]),
                ],
                &[&payer, &user, &lending_market_key, &reserve_key],
            )
            .await;

        (lending_market_f, reserve_f, obligation_f)
    }

    pub async fn load_and_deserialize<T: anchor_lang::AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> T {
        let ai = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap();

        T::try_deserialize(&mut ai.data.as_slice()).unwrap()
    }

    pub fn payer(&self) -> Pubkey {
        self.context.borrow().payer.pubkey()
    }

    pub fn payer_keypair(&self) -> Keypair {
        clone_keypair(&self.context.borrow().payer)
    }

    pub async fn set_pyth_oracle_timestamp(&self, address: Pubkey, timestamp: i64) {
        let mut ctx = self.context.borrow_mut();

        let mut account = ctx
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();

        let data = account.data.as_mut_slice();
        let mut data: SolanaPriceAccount =
            *pyth_sdk_solana::state::load_price_account(data).unwrap();

        data.timestamp = timestamp;
        data.prev_timestamp = timestamp;

        let bytes = bytemuck::bytes_of(&data);

        let mut aso = AccountSharedData::from(account);
        aso.set_data_from_slice(bytes);

        ctx.set_account(&address, &aso);
    }

    pub fn set_time(&self, timestamp: i64) {
        let clock = Clock {
            unix_timestamp: timestamp,
            ..Default::default()
        };
        self.context.borrow_mut().set_sysvar(&clock);
    }

    pub async fn get_minimum_rent_for_size(&self, size: usize) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size)
    }

    pub async fn get_latest_blockhash(&self) -> Hash {
        self.context
            .borrow_mut()
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap()
    }

    pub async fn get_slot(&self) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_root_slot()
            .await
            .unwrap()
    }

    pub async fn get_clock(&self) -> Clock {
        deserialize::<Clock>(
            &self
                .context
                .borrow_mut()
                .banks_client
                .get_account(sysvar::clock::ID)
                .await
                .unwrap()
                .unwrap()
                .data,
        )
        .unwrap()
    }

    pub async fn send_transaction(
        &self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<(), BanksClientError> {
        let mut ctx = self.context.borrow_mut();

        let mut signers = signers.to_vec();
        signers.push(&ctx.payer);

        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&ctx.payer.pubkey()),
            signers.as_slice(),
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }
}
