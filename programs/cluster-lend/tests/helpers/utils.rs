use cluster_lend::utils::pyth;
use pyth_sdk_solana::state::{
    AccountType, PriceAccount, PriceInfo, PriceStatus, Rational, MAGIC, VERSION_2,
};

use solana_program::pubkey;
use solana_program::pubkey::Pubkey;
use solana_sdk::{account::Account, signature::Keypair};

cfg_if::cfg_if! {
    if #[cfg(feature = "devnet")] {
        pub const PYTH_ID: Pubkey = pubkey!("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    } else if #[cfg(feature = "mainnet-beta")] {
        pub const PYTH_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
    } else {
        pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");
    }
}

pub fn ui_to_native(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * (10u64.pow(decimals as u32) as f64)) as u64
}

pub fn native_to_ui(native_amount: u64, decimals: u8) -> f64 {
    native_amount as f64 / 10u64.pow(decimals as u32) as f64
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

pub fn create_pyth_price_account(
    mint: Pubkey,
    ui_price: i64,
    mint_decimals: i32,
    timestamp: Option<i64>,
) -> Account {
    let native_price = ui_price * 10_i64.pow(mint_decimals as u32);
    Account {
        lamports: 1_000_000,
        data: bytemuck::bytes_of(&PriceAccount {
            prod: mint,
            agg: PriceInfo {
                conf: 0,
                price: native_price,
                status: PriceStatus::Trading,
                ..Default::default()
            },
            expo: -mint_decimals,
            prev_price: native_price,
            magic: MAGIC,
            ver: VERSION_2,
            atype: AccountType::Price as u32,
            timestamp: 0,
            ema_price: Rational {
                val: native_price,
                numer: native_price,
                denom: 1,
            },
            prev_timestamp: timestamp.unwrap_or(0),
            ema_conf: Rational {
                val: 0,
                numer: 0,
                denom: 1,
            },
            ..Default::default()
        })
        .to_vec(),
        owner: PYTH_ID,
        executable: false,
        rent_epoch: 361,
    }
}

#[macro_export]
macro_rules! assert_custom_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_program_test::BanksClientError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    _,
                    solana_program::instruction::InstructionError::Custom(n),
                ),
            ) => {
                assert_eq!(n, anchor_lang::error::ERROR_CODE_OFFSET + $matcher as u32)
            }
            _ => assert!(false),
        }
    };
}

#[macro_export]
macro_rules! assert_anchor_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_program_test::BanksClientError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    _,
                    solana_program::instruction::InstructionError::Custom(n),
                ),
            ) => {
                assert_eq!(n, $matcher as u32)
            }
            _ => assert!(false),
        }
    };
}

#[macro_export]
macro_rules! assert_program_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_sdk::transport::TransportError::TransactionError(
                solana_sdk::transaction::InstructionError(_, x),
            ) => {
                assert_eq!(x, $matcher)
            }
            _ => assert!(false),
        };
    };
}
