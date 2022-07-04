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

    #[msg("Invalid number remaining accounts")]
    InvalidRemainingAccounts,
}
