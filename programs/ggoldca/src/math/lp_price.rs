use crate::math::mul_div::MulDiv;
use anchor_lang::prelude::*;
use std::cmp::Ordering;

/// Strategy token price
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct LpPrice {
    /// Total amount of tokens to be distributed
    pub total_tokens: u64,
    /// Supply of strategy LP tokens
    pub minted_tokens: u64,
}

impl LpPrice {
    pub const SIZE: usize = 8 + 8;

    /// Transform input token amount to LP amount
    pub fn token_to_lp(&self, amount: u64) -> Result<u64> {
        if self.minted_tokens == 0 {
            Ok(amount)
        } else {
            Ok(amount.mul_div(self.minted_tokens, self.total_tokens)?)
        }
    }

    /// Transform LP amount to input token amount
    pub fn lp_to_token(&self, lp_amount: u64) -> Result<u64> {
        if self.minted_tokens == 0 {
            Ok(lp_amount)
        } else {
            Ok(lp_amount.mul_div(self.total_tokens, self.minted_tokens)?)
        }
    }
}

impl PartialEq for LpPrice {
    fn eq(&self, other: &Self) -> bool {
        let lhs = (self.total_tokens as u128)
            .checked_mul(other.minted_tokens as u128)
            .unwrap();

        let rhs = (other.total_tokens as u128)
            .checked_mul(self.minted_tokens as u128)
            .unwrap();

        lhs == rhs
    }
}

impl PartialOrd for LpPrice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let lhs = (self.total_tokens as u128)
            .checked_mul(other.minted_tokens as u128)
            .unwrap();

        let rhs = (other.total_tokens as u128)
            .checked_mul(self.minted_tokens as u128)
            .unwrap();

        lhs.partial_cmp(&rhs)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lp_price_cmp() {
        let price = LpPrice {
            minted_tokens: 10_000,
            total_tokens: 10_000,
        };

        let same_price = LpPrice {
            minted_tokens: 20_000,
            total_tokens: 20_000,
        };

        let greater_price = LpPrice {
            minted_tokens: 10_000,
            total_tokens: 15_000,
        };

        assert_eq!(price, same_price);
        assert!(greater_price > price);
    }
}
