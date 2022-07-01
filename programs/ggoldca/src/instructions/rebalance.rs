use crate::error::ErrorCode;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Token, TokenAccount};
use std::borrow::Borrow;
use whirlpool::math::{bit_math, tick_math, U256};

#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.input_token_a_mint_pubkey.as_ref(), vault_account.input_token_b_mint_pubkey.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_a_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_a_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Account<'info, TokenAccount>,

    #[account(constraint = whirlpool_program_id.key == &whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub whirlpool: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    pub current_position: PositionParams<'info>,
    pub new_position: PositionParams<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Rebalance<'info> {
    fn modify_liquidity_ctx(
        &self,
        position: &PositionParams<'info>,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: self.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: position.position.to_account_info(),
                position_token_account: position.position_token_account.to_account_info(),
                token_owner_account_a: self.vault_input_token_a_account.to_account_info(),
                token_owner_account_b: self.vault_input_token_b_account.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_lower: position.tick_array_lower.to_account_info(),
                tick_array_upper: position.tick_array_upper.to_account_info(),
            },
        )
    }

    // impl from @orca-so/whirlpools-sdk: PoolUtil/estimateLiquidityFromTokenAmounts
    fn new_liquidity_from_token_amounts(
        &self,
        token_amount_a: u64,
        token_amount_b: u64,
    ) -> Result<u128> {
        use anchor_lang_for_whirlpool::AccountDeserialize;

        let (curr_sqrt_price, curr_tick) = {
            let acc_data_slice: &[u8] = &self.whirlpool.try_borrow_data()?;
            let pool = whirlpool::state::whirlpool::Whirlpool::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            (pool.sqrt_price, pool.tick_current_index)
        };

        let (lower_tick, upper_tick) = {
            let acc_data_slice: &[u8] = &self.new_position.position.try_borrow_data()?;
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
}

#[derive(Accounts)]
pub struct PositionParams<'info> {
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

impl<'info> PositionParams<'info> {
    fn position_liquidity(&self) -> Result<u128> {
        use anchor_lang_for_whirlpool::AccountDeserialize;
        let acc_data_slice: &[u8] = &self.position.try_borrow_data()?;
        let position =
            whirlpool::state::position::Position::try_deserialize(&mut acc_data_slice.borrow())?;
        Ok(position.liquidity)
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

pub fn handler(ctx: Context<Rebalance>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let liquidity = ctx.accounts.current_position.position_liquidity()?;

    msg!("0.L {}", liquidity);
    msg!("0.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("0.B {}", ctx.accounts.vault_input_token_b_account.amount);

    whirlpool::cpi::decrease_liquidity(
        ctx.accounts
            .modify_liquidity_ctx(&ctx.accounts.current_position)
            .with_signer(signer),
        liquidity,
        0,
        0,
    )?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;
    msg!("1.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("1.B {}", ctx.accounts.vault_input_token_b_account.amount);

    let amount_a = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b = ctx.accounts.vault_input_token_b_account.amount;

    let new_liquidity = ctx
        .accounts
        .new_liquidity_from_token_amounts(amount_a, amount_b)?;

    whirlpool::cpi::increase_liquidity(
        ctx.accounts
            .modify_liquidity_ctx(&ctx.accounts.new_position)
            .with_signer(signer),
        new_liquidity,
        amount_a,
        amount_b,
    )?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;
    msg!("2.L {}", ctx.accounts.new_position.position_liquidity()?);
    msg!("2.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("2.B {}", ctx.accounts.vault_input_token_b_account.amount);

    Ok(())
}
