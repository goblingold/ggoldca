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

    #[msg("Exceeded token max")]
    ExceededTokenMax,

    #[msg("Invalid input token mint pubkey")]
    InvalidInputMint,
    #[msg("Invalid reward token mint pubkey")]
    InvalidRewardMint,

    #[msg("Position already opened")]
    PositionAlreadyOpened,
    #[msg("Position limit reached")]
    PositionLimitReached,
    #[msg("Position does not exist or is not active")]
    PositionNotActive,

    #[msg("Not enough fees generated yet")]
    NotEnoughFees,
    #[msg("Not enough rewards generated yet")]
    NotEnoughRewards,

    #[msg("Unauthorized user")]
    UnauthorizedUser,
    #[msg("Invalid swap")]
    InvalidSwap,
    #[msg("Invalid number of accounts")]
    InvalidNumberOfAccounts,
    #[msg("Invalid Fee")]
    InvalidFee,
    #[msg("Invalid Market Rewards")]
    InvalidMarketRewards,

    #[msg("Invalid swap program ID")]
    InvalidSwapProgramId,
    #[msg("Swap is not set for the current rewards")]
    SwapNotSet,

    #[msg("whirlpool: Liquidity amount must be less than i64::MAX")]
    WhirlpoolLiquidityTooHigh,
    #[msg("whirlpool: overflow while computing liquidity to token deltas")]
    WhirlpoolLiquidityToDeltasOverflow,
}
