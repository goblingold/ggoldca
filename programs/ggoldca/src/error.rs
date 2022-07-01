use anchor_lang::prelude::*;

/// Error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Math operation overflow")]
    MathOverflow,
    #[msg("Invalid number remaining accounts")]
    InvalidRemainingAccounts,
}
