use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang_for_whirlpool::AccountDeserialize;
use std::borrow::Borrow;
use whirlpool::math::{bit_math, tick_math, U256};

#[derive(Accounts)]
pub struct PositionAccounts<'info> {
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub whirlpool: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position_token_account: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub tick_array_lower: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub tick_array_upper: AccountInfo<'info>,
}

impl<'info> PositionAccounts<'info> {
    pub fn liquidity(&self) -> Result<u128> {
        let acc_data_slice: &[u8] = &self.position.try_borrow_data()?;
        let position =
            whirlpool::state::position::Position::try_deserialize(&mut acc_data_slice.borrow())?;
        Ok(position.liquidity)
    }

    // impl from @orca-so/whirlpools-sdk: PoolUtil/estimateLiquidityFromTokenAmounts
    pub fn liquidity_from_token_amounts(
        &self,
        token_amount_a: u64,
        token_amount_b: u64,
    ) -> Result<u128> {
        let (curr_sqrt_price, curr_tick) = {
            let acc_data_slice: &[u8] = &self.whirlpool.try_borrow_data()?;
            let pool = whirlpool::state::whirlpool::Whirlpool::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            (pool.sqrt_price, pool.tick_current_index)
        };

        let (lower_tick, upper_tick) = {
            let acc_data_slice: &[u8] = &self.position.try_borrow_data()?;
            let position = whirlpool::state::position::Position::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            (position.tick_lower_index, position.tick_upper_index)
        };

        let lower_sqrt_price = tick_math::sqrt_price_from_tick_index(lower_tick);
        let upper_sqrt_price = tick_math::sqrt_price_from_tick_index(upper_tick);

        if curr_tick >= upper_tick {
            Ok(est_liquidity_for_token_b(
                upper_sqrt_price,
                lower_sqrt_price,
                token_amount_b,
            )?)
        } else if curr_tick < lower_tick {
            Ok(est_liquidity_for_token_a(
                lower_sqrt_price,
                upper_sqrt_price,
                token_amount_a,
            )?)
        } else {
            let est_liquidity_amount_a =
                est_liquidity_for_token_a(curr_sqrt_price, upper_sqrt_price, token_amount_a)?;
            let est_liquidity_amount_b =
                est_liquidity_for_token_b(curr_sqrt_price, lower_sqrt_price, token_amount_b)?;

            Ok(std::cmp::min(
                est_liquidity_amount_a,
                est_liquidity_amount_b,
            ))
        }
    }

    // impl from @orca-so/whirlpools-sdk: PoolUtil/getTokenAmountsFromLiquidity
    pub fn token_amounts_from_liquidity(self, liquidity: u128) -> Result<(u64, u64)> {
        let current_price = {
            let acc_data_slice: &[u8] = &self.whirlpool.try_borrow_data()?;
            let pool = whirlpool::state::whirlpool::Whirlpool::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            U256::from(pool.sqrt_price)
        };

        let (lower_price, upper_price) = {
            let acc_data_slice: &[u8] = &self.position.try_borrow_data()?;
            let position = whirlpool::state::position::Position::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            let lower_tick = position.tick_lower_index;
            let upper_tick = position.tick_upper_index;
            (
                U256::from(tick_math::sqrt_price_from_tick_index(lower_tick)),
                U256::from(tick_math::sqrt_price_from_tick_index(upper_tick)),
            )
        };

        let liquidity = U256::from(liquidity);

        let token_a;
        let token_b;
        if current_price < lower_price {
            // x = L * (pb - pa) / (pa * pb)
            token_a = (liquidity << bit_math::Q64_RESOLUTION)
                .checked_mul(
                    upper_price
                        .checked_sub(lower_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?
                .checked_div(
                    upper_price
                        .checked_mul(lower_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?;
            token_b = U256::from(0);
        } else if current_price < upper_price {
            token_a = (liquidity << bit_math::Q64_RESOLUTION)
                .checked_mul(
                    upper_price
                        .checked_sub(current_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?
                .checked_div(
                    upper_price
                        .checked_mul(current_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?;
            token_b = liquidity
                .checked_mul(
                    current_price
                        .checked_sub(lower_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?
                >> bit_math::Q64_RESOLUTION;
        } else {
            token_a = U256::from(0);
            token_b = liquidity
                .checked_mul(
                    upper_price
                        .checked_sub(lower_price)
                        .ok_or_else(|| error!(ErrorCode::MathOverflow))?,
                )
                .ok_or_else(|| error!(ErrorCode::MathOverflow))?
                >> bit_math::Q64_RESOLUTION;
        };

        //TODO ensure round-up
        let token_a = token_a
            .try_into_u64()
            .map_err(|_| error!(ErrorCode::MathOverflow))?;

        let token_b = token_b
            .try_into_u64()
            .map_err(|_| error!(ErrorCode::MathOverflow))?;

        Ok((token_a, token_b))
    }
}

// impl from @orca-so/whirlpools-sdk: PoolUtil/estLiquidityForTokenA
fn est_liquidity_for_token_a(
    sqrt_price_1: u128,
    sqrt_price_2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price_x64 = U256::from(std::cmp::min(sqrt_price_1, sqrt_price_2));
    let upper_sqrt_price_x64 = U256::from(std::cmp::max(sqrt_price_1, sqrt_price_2));

    let num = U256::from(token_amount)
        .checked_mul(upper_sqrt_price_x64)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?
        .checked_mul(lower_sqrt_price_x64)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?
        >> bit_math::Q64_RESOLUTION;

    let den = upper_sqrt_price_x64
        .checked_sub(lower_sqrt_price_x64)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?;

    num.checked_div(den)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?
        .try_into_u128()
        .map_err(|_| error!(ErrorCode::MathOverflow))
}

// impl from @orca-so/whirlpools-sdk: PoolUtil/estLiquidityForTokenB
fn est_liquidity_for_token_b(
    sqrt_price_1: u128,
    sqrt_price_2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price_x64 = std::cmp::min(sqrt_price_1, sqrt_price_2);
    let upper_sqrt_price_x64 = std::cmp::max(sqrt_price_1, sqrt_price_2);

    let delta = upper_sqrt_price_x64
        .checked_sub(lower_sqrt_price_x64)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?;

    let token_amount_x64 = u128::from(token_amount) << bit_math::Q64_RESOLUTION;

    token_amount_x64
        .checked_div(delta)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))
}
