use crate::error::ErrorCode;
use crate::math::safe_arithmetics::SafeArithmetics;
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

        est_liquidity_from_token_amounts(
            curr_sqrt_price,
            curr_tick,
            lower_tick,
            upper_tick,
            token_amount_a,
            token_amount_b,
        )
    }

    pub fn token_amounts_from_liquidity(&self, liquidity: u128) -> Result<(u64, u64)> {
        self.token_amounts_from_liquidity_is_round(liquidity, false)
    }

    pub fn token_amounts_from_liquidity_round_up(&self, liquidity: u128) -> Result<(u64, u64)> {
        self.token_amounts_from_liquidity_is_round(liquidity, true)
    }

    fn token_amounts_from_liquidity_is_round(
        &self,
        liquidity: u128,
        round_up: bool,
    ) -> Result<(u64, u64)> {
        let current_sqrt_price = {
            let acc_data_slice: &[u8] = &self.whirlpool.try_borrow_data()?;
            let pool = whirlpool::state::whirlpool::Whirlpool::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            pool.sqrt_price
        };

        let (lower_sqrt_price, upper_sqrt_price) = {
            let acc_data_slice: &[u8] = &self.position.try_borrow_data()?;
            let position = whirlpool::state::position::Position::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            (
                tick_math::sqrt_price_from_tick_index(position.tick_lower_index),
                tick_math::sqrt_price_from_tick_index(position.tick_upper_index),
            )
        };

        get_token_amounts_from_liquidity(
            liquidity,
            current_sqrt_price,
            lower_sqrt_price,
            upper_sqrt_price,
            round_up,
        )
    }
}

// impl from @orca-so/whirlpools-sdk: PoolUtil/estimateLiquidityFromTokenAmounts
fn est_liquidity_from_token_amounts(
    curr_sqrt_price: u128,
    curr_tick: i32,
    lower_tick: i32,
    upper_tick: i32,
    token_amount_a: u64,
    token_amount_b: u64,
) -> Result<u128> {
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

// impl from @orca-so/whirlpools-sdk: PoolUtil/estLiquidityForTokenA
fn est_liquidity_for_token_a(
    sqrt_price_1: u128,
    sqrt_price_2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price_x64 = U256::from(std::cmp::min(sqrt_price_1, sqrt_price_2));
    let upper_sqrt_price_x64 = U256::from(std::cmp::max(sqrt_price_1, sqrt_price_2));

    let num = U256::from(token_amount)
        .safe_mul(upper_sqrt_price_x64)?
        .safe_mul(lower_sqrt_price_x64)?
        >> bit_math::Q64_RESOLUTION;

    let den = upper_sqrt_price_x64.safe_sub(lower_sqrt_price_x64)?;

    num.safe_div(den)?
        .try_into_u128()
        .map_err(|_| error!(ErrorCode::MathOverflowConversion))
}

// impl from @orca-so/whirlpools-sdk: PoolUtil/estLiquidityForTokenB
fn est_liquidity_for_token_b(
    sqrt_price_1: u128,
    sqrt_price_2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price_x64 = std::cmp::min(sqrt_price_1, sqrt_price_2);
    let upper_sqrt_price_x64 = std::cmp::max(sqrt_price_1, sqrt_price_2);

    let delta = upper_sqrt_price_x64.safe_sub(lower_sqrt_price_x64)?;
    let token_amount_x64 = u128::from(token_amount) << bit_math::Q64_RESOLUTION;

    token_amount_x64.safe_div(delta)
}

// impl from @orca-so/whirlpools-sdk: PoolUtil/getTokenAmountsFromLiquidity
fn get_token_amounts_from_liquidity(
    liquidity: u128,
    current_sqrt_price: u128,
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
    round_up: bool,
) -> Result<(u64, u64)> {
    let token_a;
    let token_b;

    let liquidity = U256::from(liquidity);
    let current_sqrt_price = U256::from(current_sqrt_price);
    let lower_sqrt_price = U256::from(lower_sqrt_price);
    let upper_sqrt_price = U256::from(upper_sqrt_price);

    if current_sqrt_price < lower_sqrt_price {
        token_a = {
            let mut numerator = (liquidity << bit_math::Q64_RESOLUTION)
                .safe_mul(upper_sqrt_price.safe_sub(lower_sqrt_price)?)?;

            let denominator = upper_sqrt_price.safe_mul(lower_sqrt_price)?;

            if round_up {
                numerator = numerator.safe_add(denominator)?.safe_sub(1.into())?;
            }

            numerator.safe_div(denominator)?
        };

        token_b = U256::from(0);
    } else if current_sqrt_price < upper_sqrt_price {
        token_a = {
            let mut numerator = (liquidity << bit_math::Q64_RESOLUTION)
                .safe_mul(upper_sqrt_price.safe_sub(current_sqrt_price)?)?;

            let denominator = upper_sqrt_price.safe_mul(current_sqrt_price)?;

            if round_up {
                numerator = numerator.safe_add(denominator)?.safe_sub(1.into())?;
            }
            numerator.safe_div(denominator)?
        };
        token_b = {
            let x64 = liquidity.safe_mul(current_sqrt_price.safe_sub(lower_sqrt_price)?)?;
            let result = x64 >> bit_math::Q64_RESOLUTION;

            if round_up && (x64 & U256::from(u64::MAX) > 0.into()) {
                result.safe_add(1.into())?
            } else {
                result
            }
        };
    } else {
        token_a = U256::from(0);
        token_b = {
            let x64 = liquidity.safe_mul(upper_sqrt_price.safe_sub(lower_sqrt_price)?)?;
            let result = x64 >> bit_math::Q64_RESOLUTION;

            if round_up && (x64 & U256::from(u64::MAX) > 0.into()) {
                result.safe_add(1.into())?
            } else {
                result
            }
        };
    };

    let token_a = token_a
        .try_into_u64()
        .map_err(|_| error!(ErrorCode::MathOverflowConversion))?;

    let token_b = token_b
        .try_into_u64()
        .map_err(|_| error!(ErrorCode::MathOverflowConversion))?;

    Ok((token_a, token_b))
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestData {
        pub curr_tick: i32,
        pub lower_tick: i32,
        pub upper_tick: i32,
        pub liquidity: u128,
        pub token_amount_a: u64,
        pub token_amount_b: u64,
    }

    macro_rules! gen_tests {
        ($data:expr) => {
            #[test]
            fn test_est_liquidity_from_token_amounts() {
                let liquidity = est_liquidity_from_token_amounts(
                    tick_math::sqrt_price_from_tick_index($data.curr_tick),
                    $data.curr_tick,
                    $data.lower_tick,
                    $data.upper_tick,
                    $data.token_amount_a,
                    $data.token_amount_b,
                )
                .unwrap();

                assert_eq!(liquidity, $data.liquidity);
            }

            #[test]
            fn test_get_token_amounts_from_liquidity() {
                let amounts = get_token_amounts_from_liquidity(
                    $data.liquidity,
                    tick_math::sqrt_price_from_tick_index($data.curr_tick),
                    tick_math::sqrt_price_from_tick_index($data.lower_tick),
                    tick_math::sqrt_price_from_tick_index($data.upper_tick),
                    false,
                )
                .unwrap();

                assert_eq!(
                    amounts,
                    (
                        $data.token_amount_a.saturating_sub(1),
                        $data.token_amount_b.saturating_sub(1)
                    )
                )
            }

            #[test]
            fn test_get_token_amounts_from_liquidity_round_up() {
                let amounts = get_token_amounts_from_liquidity(
                    $data.liquidity,
                    tick_math::sqrt_price_from_tick_index($data.curr_tick),
                    tick_math::sqrt_price_from_tick_index($data.lower_tick),
                    tick_math::sqrt_price_from_tick_index($data.upper_tick),
                    true,
                )
                .unwrap();

                assert_eq!(amounts, ($data.token_amount_a, $data.token_amount_b))
            }
        };
    }

    mod case_1 {
        use super::*;
        gen_tests! { TestData {
            curr_tick: 0,
            lower_tick: -1280,
            upper_tick: 1280,
            token_amount_a: 167_000,
            token_amount_b: 167_000,
            liquidity: 2693896,
        }}
    }

    mod case_2 {
        use super::*;
        gen_tests! { TestData {
            curr_tick: 500,
            lower_tick: 7168,
            upper_tick: 8960,
            token_amount_a: 1_000_000,
            token_amount_b: 0,
            liquidity:  16698106,
        }}
    }

    mod case_3 {
        use super::*;
        gen_tests! { TestData {
            curr_tick: 1300,
            lower_tick: -1280,
            upper_tick: 1280,
            token_amount_a: 0,
            token_amount_b: 167_000,
            liquidity: 1303862,
        }}
    }

    mod case_4 {
        use super::*;
        gen_tests! { TestData {
            curr_tick: -443621,
            lower_tick: -443632,
            upper_tick: -443624,
            token_amount_a: 0,
            token_amount_b: u64::MAX,
            liquidity: 197997328626229089162140962642757,
        }}
    }
}
