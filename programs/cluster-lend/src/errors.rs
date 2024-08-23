use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Market authority is invalid")]
    InvalidMarketAuthority,
    #[msg("Market owner is invalid")]
    InvalidMarketOwner,
    #[msg("Input account owner is not the program address")]
    InvalidAccountOwner,
    #[msg("Input amount is invalid")]
    InvalidAmount,
    #[msg("Input config value is invalid")]
    InvalidConfig,
    #[msg("Input account must be a signer")]
    InvalidSigner,
    #[msg("Invalid account input")]
    InvalidAccountInput,
    #[msg("Math operation overflow")]
    MathOverflow,
    #[msg("Insufficient liquidity available")]
    InsufficientLiquidity,
    #[msg("Reserve state needs to be refreshed")]
    ReserveStale,
    #[msg("Withdraw amount too small")]
    WithdrawTooSmall,
    #[msg("Withdraw amount too large")]
    WithdrawTooLarge,
    #[msg("Borrow amount too small to receive liquidity after fees")]
    BorrowTooSmall,
    #[msg("Borrow amount too large for deposited collateral")]
    BorrowTooLarge,
    #[msg("Repay amount too small to transfer liquidity")]
    RepayTooSmall,
    #[msg("Liquidation amount too small to receive collateral")]
    LiquidationTooSmall,
    #[msg("Cannot liquidate healthy obligations")]
    ObligationHealthy,
    #[msg("Obligation state needs to be refreshed")]
    ObligationStale,
    #[msg("Obligation reserve limit exceeded")]
    ObligationReserveLimit,
    #[msg("Obligation owner is invalid")]
    InvalidObligationOwner,
    #[msg("Obligation deposits are empty")]
    ObligationDepositsEmpty,
    #[msg("Obligation borrows are empty")]
    ObligationBorrowsEmpty,
    #[msg("Obligation deposits have zero value")]
    ObligationDepositsZero,
    #[msg("Obligation borrows have zero value")]
    ObligationBorrowsZero,
    #[msg("Invalid obligation collateral")]
    InvalidObligationCollateral,
    #[msg("Invalid obligation liquidity")]
    InvalidObligationLiquidity,
    #[msg("Obligation collateral is empty")]
    ObligationCollateralEmpty,
    #[msg("Obligation liquidity is empty")]
    ObligationLiquidityEmpty,
    #[msg("Interest rate is negative")]
    NegativeInterestRate,
    #[msg("Input oracle config is invalid")]
    InvalidOracleConfig,
    #[msg("Insufficient protocol fees to claim or no liquidity available")]
    InsufficientProtocolFeesToRedeem,
    #[msg("No cpi flash borrows allowed")]
    FlashBorrowCpi,
    #[msg("No corresponding repay found for flash borrow")]
    NoFlashRepayFound,
    #[msg("Invalid repay found")]
    InvalidFlashRepay,
    #[msg("No cpi flash repays allowed")]
    FlashRepayCpi,
    #[msg("Multiple flash borrows not allowed in the same transaction")]
    MultipleFlashBorrows,
    #[msg("Flash loans are disabled for this reserve")]
    FlashLoansDisabled,
    #[msg("Price too old")]
    PriceTooOld,
    #[msg("Price too divergent from twap")]
    PriceTooDivergentFromTwap,
    #[msg("Invalid twap price")]
    InvalidTwapPrice,
    #[msg("Emergency mode is enabled")]
    GlobalEmergencyMode,
    #[msg("Invalid lending market config")]
    InvalidFlag,
    #[msg("Price is not valid")]
    PriceNotValid,
    #[msg("Price is zero")]
    PriceIsZero,
    #[msg("Price confidence too wide")]
    PriceConfidenceTooWide,
    #[msg("Conversion between integers failed")]
    IntegerOverflow,
    #[msg("Wrong instruction at expected position")]
    IncorrectInstructionInPosition,
    #[msg("No price found")]
    NoPriceFound,
    #[msg("Invalid Twap configuration: Twap is enabled but one of the enabled price doesn't have a twap")]
    InvalidTwapConfig,
    #[msg("Pyth price account does not match configuration")]
    InvalidPythPriceAccount,
    #[msg("The obligation has one collateral with an LTV set to 0. Withdraw it before withdrawing other collaterals")]
    ObligationCollateralLtvZero,
    #[msg("Seeds must be default pubkeys for tag 0, and mint addresses for tag 1 or 2")]
    InvalidObligationSeedsValue,
    #[msg("Obligation id must be 0")]
    InvalidObligationId,
    #[msg("Invalid borrow rate curve point")]
    InvalidBorrowRateCurvePoint,
    #[msg("Invalid utilization rate")]
    InvalidUtilizationRate,
    #[msg("Obligation hasn't been fully liquidated and debt cannot be socialized.")]
    CannotSocializeObligationWithCollateral,
    #[msg("Obligation has no borrows or deposits.")]
    ObligationEmpty,
    #[msg("Withdrawal cap is reached")]
    WithdrawalCapReached,
    #[msg("The last interval start timestamp is greater than the current timestamp")]
    LastTimestampGreaterThanCurrent,
    #[msg("The reward amount is less than the minimum acceptable received collateral")]
    LiquidationSlippageError,
    #[msg("Isolated Asset Tier Violation")]
    IsolatedAssetTierViolation,
    #[msg("Reserve was deprecated, no longer usable")]
    ReserveDeprecated,
    #[msg("CPI disabled for this instruction")]
    CpiDisabled,
    #[msg("Reserve is marked as obsolete")]
    ReserveObsolete,
    #[msg("Obligation has a deposit in a deprecated reserve")]
    ObligationInDeprecatedReserve,
    #[msg("This collateral cannot be liquidated (LTV set to 0)")]
    CollateralNonLiquidatable,
    #[msg("Borrowing is disabled")]
    BorrowingDisabled,
    #[msg("Cannot borrow above borrow limit")]
    BorrowLimitExceeded,
    #[msg("Cannot deposit above deposit limit")]
    DepositLimitExceeded,
    #[msg("Net value remaining too small")]
    NetValueRemainingTooSmall,
    #[msg("Cannot get the obligation in a worse position")]
    WorseLTVBlocked,
    #[msg("Cannot have more liabilities than assets in a position")]
    LiabilitiesBiggerThanAssets,
    #[msg("Reserve state and token account cannot drift")]
    ReserveTokenBalanceMismatch,
    #[msg("Reserve token account has been unexpectedly modified")]
    ReserveVaultBalanceMismatch,
    #[msg("Reserve internal state accounting has been unexpectedly modified")]
    ReserveAccountingMismatch,
}

pub type LendingResult<T = ()> = std::result::Result<T, LendingError>;
