#[cfg(test)]
mod helpers;

use cluster_lend::{utils::pda, InitObligationArgs, Obligation};
use lending_market::LendingMarketFixture;

use obligation::ObligationFixture;
use solana_program_test::*;

use helpers::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use test::{TestFixture, USDC_QUOTE_CURRENCY};

#[tokio::test]
async fn success_init_update_obligation() {
    let test_f = TestFixture::new().await;

    let payer = test_f.payer_keypair();

    let lending_market_key = Keypair::new();
    let lending_market_f = LendingMarketFixture {
        key: lending_market_key.pubkey(),
        owner: payer.pubkey(),
    };

    let init_obligation_args = InitObligationArgs { tag: 0, id: 0 };
    let obligation_key = pda::init_obligation_pda(
        &payer.pubkey(),
        &lending_market_f.key,
        &Pubkey::default(),
        &Pubkey::default(),
        &init_obligation_args,
    );
    let obligation_f = ObligationFixture {
        key: obligation_key,
        owner: payer.pubkey(),
        payer: payer.pubkey(),
        lending_market: lending_market_f.key,
    };

    let r = test_f
        .send_transaction(
            &[
                lending_market_f.init_market_ix(USDC_QUOTE_CURRENCY),
                obligation_f.initialize_obligation_ix(init_obligation_args),
            ],
            &[&payer, &lending_market_key],
        )
        .await;
    assert!(r.is_ok());

    // Fetch obligation account
    let obligation: Obligation = test_f.load_and_deserialize(&obligation_f.key).await;

    // Check properties
    assert_eq!(obligation.lending_market, lending_market_key.pubkey());
    assert_eq!(obligation.tag, 0);

    // try to refresh obligation
    let r = test_f
        .send_transaction(&[obligation_f.refresh_ix(vec![])], &[&payer])
        .await;
    assert!(r.is_ok());
}
