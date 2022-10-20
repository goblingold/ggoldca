use anchor_lang::prelude::*;

/// Error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Math overflow during add")]
    MathOverflowAdd,
    #[msg("Math overflow during sub")]
    MathOverflowSub,
    #[msg("Math overflow during mul")]
    MathOverflowMul,
    #[msg("Math division by zero")]
    MathZeroDivision,
    #[msg("Math overflow during type conversion")]
    MathOverflowConversion,

    #[msg("Invalid vault version")]
    InvalidVaultVersion,

    #[msg("Unauthorized user")]
    UnauthorizedUser,
    #[msg("The smart contract is paused")]
    PausedSmartContract,
    #[msg("The provided vault is paused")]
    PausedVault,
    #[msg("Not enough elapsed slots since last call")]
    NotEnoughSlots,

    #[msg("Fee cannot exceed FEE_SCALE")]
    InvalidFee,

    #[msg("Market rewards input invalid destination account mint")]
    MarketInvalidDestination,
    #[msg("Market rewards input tokens not allowed")]
    MarketInvalidMint,
    #[msg("Market rewards input zero min_amount_out not allowed")]
    MarketInvalidZeroAmount,

    #[msg("LP amount must be greater than zero")]
    ZeroLpAmount,
    #[msg("Exceeded token max")]
    ExceededTokenMax,

    #[msg("Invalid destination token account")]
    InvalidDestinationAccount,
    #[msg("Invalid input token mint pubkey")]
    InvalidInputMint,
    #[msg("Invalid reward token mint pubkey")]
    InvalidRewardMint,

    #[msg("Position already opened")]
    PositionAlreadyOpened,
    #[msg("Position limit reached")]
    PositionLimitReached,
    #[msg("Position is not active")]
    PositionNotActive,
    #[msg("Position does not exist")]
    PositionNonExistence,

    #[msg("Cannot rebalance into the active position")]
    RebalanceIntoActivePosition,
    #[msg("Missing reinvest instruction after rebalance")]
    MissingReinvest,
    #[msg("Invalid instruction data")]
    InvalidIxData,

    #[msg("Not enough fees generated yet")]
    NotEnoughFees,
    #[msg("Not enough rewards generated yet")]
    NotEnoughRewards,

    #[msg("Invalid number of accounts")]
    InvalidNumberOfAccounts,

    #[msg("Swap is not set for the current rewards")]
    SwapNotSet,
    #[msg("Invalid swap program ID")]
    SwapInvalidProgramId,

    #[msg("Transfer is not set for the current rewards")]
    TransferNotSet,

    #[msg("whirlpool_cpi: Liquidity amount must be less than i64::MAX")]
    WhirlpoolLiquidityTooHigh,
    #[msg("whirlpool_cpi: Overflow while computing liquidity to token deltas")]
    WhirlpoolLiquidityToDeltasOverflow,
}
