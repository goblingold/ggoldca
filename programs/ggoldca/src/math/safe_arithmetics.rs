use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use whirlpool::math::U256;

pub trait SafeArithmetics {
    type Output;
    fn safe_add(&self, rhs: Self) -> Result<Self::Output>;
    fn safe_sub(&self, rhs: Self) -> Result<Self::Output>;
    fn safe_mul(&self, rhs: Self) -> Result<Self::Output>;
    fn safe_div(&self, rhs: Self) -> Result<Self::Output>;
}

macro_rules! impl_safe_arithmetics {
    ($type:ty) => {
        impl SafeArithmetics for $type {
            type Output = $type;

            fn safe_add(&self, rhs: Self) -> Result<Self> {
                self.checked_add(rhs)
                    .ok_or_else(|| error!(ErrorCode::MathOverflow))
            }

            fn safe_sub(&self, rhs: Self) -> Result<Self> {
                self.checked_sub(rhs)
                    .ok_or_else(|| error!(ErrorCode::MathOverflow))
            }

            fn safe_mul(&self, rhs: Self) -> Result<Self> {
                self.checked_mul(rhs)
                    .ok_or_else(|| error!(ErrorCode::MathOverflow))
            }

            fn safe_div(&self, rhs: Self) -> Result<Self> {
                self.checked_div(rhs)
                    .ok_or_else(|| error!(ErrorCode::MathOverflow))
            }
        }
    };
}

impl_safe_arithmetics!(u128);
impl_safe_arithmetics!(U256);
