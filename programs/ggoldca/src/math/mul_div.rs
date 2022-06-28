use crate::error::ErrorCode;
use anchor_lang::prelude::*;

/// Trait to checked multiply and divide a number in an intermediate higher representation
pub trait MulDiv {
    fn mul_div(self, num: Self, div: Self) -> Result<Self>
    where
        Self: Sized;
}

impl MulDiv for u64 {
    fn mul_div(self, num: Self, div: Self) -> Result<Self> {
        Ok(u128::from(self)
            .checked_mul(u128::from(num))
            .ok_or_else(|| error!(ErrorCode::MathOverflow))?
            .checked_div(u128::from(div))
            .ok_or_else(|| error!(ErrorCode::MathOverflow))?
            .try_into()
            .map_err(|_| ErrorCode::MathOverflow)?)
    }
}
