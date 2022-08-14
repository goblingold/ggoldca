use crate::error::ErrorCode;
use crate::instructions::swap_rewards::SwapEvent;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Mint, Token, TokenAccount};
use whirlpool::math::{
    bit_math,
    tick_math::{MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64},
    U256,
};

#[event]
struct ReinvestEvent {
    vault_account: Pubkey,
    lp_supply: u64,
    liquidity: u128,
    liquidity_increase: u128,
}

#[derive(Accounts)]
pub struct Reinvest<'info> {
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(
        mint::authority = vault_account.key(),
        seeds = [VAULT_LP_TOKEN_MINT_SEED, vault_account.key().as_ref()],
        bump = vault_account.bumps.lp_token_mint
    )]
    pub vault_lp_token_mint_pubkey: Account<'info, Mint>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = vault_account.input_token_a_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    #[account(
        constraint = position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = position.position.key() == vault_account.active_position_key() @ ErrorCode::PositionNotActive,
    )]
    pub position: PositionAccounts<'info>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub tick_array_0: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub tick_array_1: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub tick_array_2: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub oracle: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Reinvest<'info> {
    fn swap_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::Swap<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::Swap {
                token_program: self.token_program.to_account_info(),
                token_authority: self.vault_account.to_account_info(),
                whirlpool: self.position.whirlpool.to_account_info(),
                token_owner_account_a: self.vault_input_token_a_account.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_owner_account_b: self.vault_input_token_b_account.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_0: self.tick_array_0.to_account_info(),
                tick_array_1: self.tick_array_1.to_account_info(),
                tick_array_2: self.tick_array_2.to_account_info(),
                oracle: self.oracle.to_account_info(),
            },
        )
    }

    fn modify_liquidity_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: self.position.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                token_owner_account_a: self.vault_input_token_a_account.to_account_info(),
                token_owner_account_b: self.vault_input_token_b_account.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_lower: self.position.tick_array_lower.to_account_info(),
                tick_array_upper: self.position.tick_array_upper.to_account_info(),
            },
        )
    }

    fn deposit_max_possible_liquidity_cpi(&self, signer: &[&[&[u8]]]) -> Result<()> {
        let amount_a = self.vault_input_token_a_account.amount;
        let amount_b = self.vault_input_token_b_account.amount;

        if amount_a > 0 && amount_b > 0 {
            let liquidity = self
                .position
                .liquidity_from_token_amounts(amount_a, amount_b)?;

            whirlpool::cpi::increase_liquidity(
                self.modify_liquidity_ctx().with_signer(signer),
                liquidity,
                amount_a,
                amount_b,
            )?;
        };

        Ok(())
    }
}

pub fn handler(ctx: Context<Reinvest>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let liquidity_before = ctx.accounts.position.liquidity()?;

    // Swap some tokens in order to maintain the position ratio
    {
        let amount_a = ctx.accounts.vault_input_token_a_account.amount;
        let amount_b = ctx.accounts.vault_input_token_b_account.amount;

        let (position_amount_a, position_amount_b) = ctx
            .accounts
            .position
            .token_amounts_from_liquidity(ctx.accounts.position.liquidity()?)?;

        let price_x128: U256 = {
            use anchor_lang_for_whirlpool::AccountDeserialize;
            use std::borrow::Borrow;

            let acc_data_slice: &[u8] = &ctx.accounts.position.whirlpool.try_borrow_data()?;
            let pool = whirlpool::state::whirlpool::Whirlpool::try_deserialize(
                &mut acc_data_slice.borrow(),
            )?;

            U256::from(pool.sqrt_price).pow(2.into())
        };

        let ratio_x64 = (1_u128 << bit_math::Q64_RESOLUTION)
            .safe_mul_div(position_amount_a.into(), position_amount_b.into())?;

        let ratio_x192 =
            U256::from(ratio_x64) << bit_math::Q64_RESOLUTION << bit_math::Q64_RESOLUTION;

        let ratio_to_price_x64 = ratio_x192
            .safe_div(price_x128)?
            .try_into_u128()
            .map_err(|_| error!(ErrorCode::MathOverflowConversion))?;

        let ratio_amount_b_x64 = ratio_x64.safe_mul(amount_b.into())?;
        let amount_a_x64 = u128::from(amount_a) << bit_math::Q64_RESOLUTION;

        let numerator = if amount_a_x64 > ratio_amount_b_x64 {
            amount_a_x64.safe_sub(ratio_amount_b_x64)?
        } else {
            ratio_amount_b_x64.safe_sub(amount_a_x64)?
        };

        let amount_to_swap: u64 = numerator
            .safe_div((1_u128 << bit_math::Q64_RESOLUTION).safe_add(ratio_to_price_x64)?)?
            .try_into()
            .map_err(|_| error!(ErrorCode::MathOverflowConversion))?;

        let (sqrt_price_limit, is_swap_from_a_to_b) = if amount_a_x64 > ratio_amount_b_x64 {
            (MIN_SQRT_PRICE_X64, true)
        } else {
            (MAX_SQRT_PRICE_X64, false)
        };

        whirlpool::cpi::swap(
            ctx.accounts.swap_ctx().with_signer(signer),
            amount_to_swap,
            1,
            sqrt_price_limit,
            true,
            is_swap_from_a_to_b,
        )?;

        ctx.accounts.vault_input_token_a_account.reload()?;
        ctx.accounts.vault_input_token_b_account.reload()?;

        let event = if is_swap_from_a_to_b {
            SwapEvent {
                vault_account: ctx.accounts.vault_account.key(),
                mint_in: ctx.accounts.vault_input_token_a_account.mint,
                amount_in: amount_to_swap,
                mint_out: ctx.accounts.vault_input_token_b_account.mint,
                amount_out: ctx
                    .accounts
                    .vault_input_token_b_account
                    .amount
                    .safe_sub(amount_b)?,
            }
        } else {
            SwapEvent {
                vault_account: ctx.accounts.vault_account.key(),
                mint_in: ctx.accounts.vault_input_token_b_account.mint,
                amount_in: amount_to_swap,
                mint_out: ctx.accounts.vault_input_token_a_account.mint,
                amount_out: ctx
                    .accounts
                    .vault_input_token_a_account
                    .amount
                    .safe_sub(amount_a)?,
            }
        };

        emit!(event);
    }

    ctx.accounts.deposit_max_possible_liquidity_cpi(signer)?;

    let liquidity_after = ctx.accounts.position.liquidity()?;
    let liquidity_increase = liquidity_after.safe_sub(liquidity_before)?;
    ctx.accounts.vault_account.last_liquidity_increase = liquidity_increase;

    emit!(ReinvestEvent {
        vault_account: ctx.accounts.vault_account.key(),
        lp_supply: ctx.accounts.vault_lp_token_mint_pubkey.supply,
        liquidity: liquidity_after,
        liquidity_increase,
    });

    Ok(())
}
