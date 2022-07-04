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

impl_safe_arithmetics!(u64);
impl_safe_arithmetics!(u128);
impl_safe_arithmetics!(U256);

pub trait SafeMulDiv: Sized {
    type Output;

    fn safe_mul_div(&self, mul: Self, div: Self) -> Result<<Self as SafeMulDiv>::Output> {
        self.safe_mul_div_is_round(mul, div, false)
    }

    fn safe_mul_div_round_up(&self, mul: Self, div: Self) -> Result<<Self as SafeMulDiv>::Output> {
        self.safe_mul_div_is_round(mul, div, true)
    }

    fn safe_mul_div_is_round(
        &self,
        mul: Self,
        div: Self,
        is_round: bool,
    ) -> Result<<Self as SafeMulDiv>::Output>;
}

impl SafeMulDiv for u64 {
    type Output = u64;

    fn safe_mul_div_is_round(&self, mul: Self, div: Self, is_round: bool) -> Result<Self> {
        let mut num = u128::from(*self).safe_mul(u128::from(mul))?;
        let div_128 = u128::from(div);

        if is_round {
            num = num.safe_add(div_128)?.safe_sub(1)?;
        }

        Ok(num
            .safe_div(div_128)?
            .try_into()
            .map_err(|_| ErrorCode::MathOverflow)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_safe_mul_div() {
        let number = 11;
        let multiplier = 5;
        let divisor = 4;

        let result = number.safe_mul_div(multiplier, divisor).unwrap();
        let result_up = number.safe_mul_div_round_up(multiplier, divisor).unwrap();

        let expected_floor = 13;
        let expected_ceil = 14;

        assert_eq!(result, expected_floor);
        assert_eq!(result_up, expected_ceil);
    }

    #[test]
    fn test_safe_mul_div_2() {
        let number = 56;
        let multiplier = 23;
        let divisor = 3;

        let result = number.safe_mul_div(multiplier, divisor).unwrap();
        let result_up = number.safe_mul_div_round_up(multiplier, divisor).unwrap();

        let expected_floor = 429;
        let expected_ceil = 430;

        assert_eq!(result, expected_floor);
        assert_eq!(result_up, expected_ceil);
    }

    #[test]
    fn test_safe_mul_div_3() {
        let number = 8;
        let multiplier = 3;
        let divisor = 2;

        let result = number.safe_mul_div(multiplier, divisor).unwrap();
        let result_up = number.safe_mul_div_round_up(multiplier, divisor).unwrap();

        let expected_floor = 12;
        let expected_ceil = 12;

        assert_eq!(result, expected_floor);
        assert_eq!(result_up, expected_ceil);
    }
}
