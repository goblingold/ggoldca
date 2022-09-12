use crate::error::ErrorCode;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::{LpPriceAccount, VaultAccount};
use crate::{
    VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED, VAULT_LP_TOKEN_PRICE_SEED, VAULT_VERSION,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::{
    context::CpiContext as CpiContextForWhirlpool, AccountDeserialize,
};
use anchor_spl::token::{self, Approve, Burn, Mint, MintTo, Revoke, Token, TokenAccount, Transfer};
use std::borrow::Borrow;
use switchboard_v2::AggregatorAccountData;
use whirlpool::{
    math::convert_to_liquidity_delta,
    state::{Position, Whirlpool},
};

#[derive(Accounts)]
pub struct UpdateLpPrice<'info> {
    #[account(
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
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidLpPriceVersion,
        seeds = [VAULT_LP_TOKEN_PRICE_SEED, vault_account.key().as_ref()],
        bump = lp_price_account.bump
    )]
    pub lp_price_account: Account<'info, LpPriceAccount>,
    #[account(
        associated_token::mint = vault_account.input_token_a_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_a_account: Account<'info, TokenAccount>,
    #[account(
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Account<'info, TokenAccount>,

    #[account(address = vault_account.whirlpool_id)]
    /// CHECK: address is checked
    pub whirlpool: AccountInfo<'info>,
    #[account(address = vault_account.active_position_key() @ ErrorCode::PositionNotActive)]
    /// CHECK: address is checked
    pub position: AccountInfo<'info>,

    #[account(address = lp_price_account.switchboard_price_token_a_pubkey)]
    /// CHECK: address is checked
    pub switchboard_price_a: AccountInfo<'info>,
    #[account(address = lp_price_account.switchboard_price_token_b_pubkey)]
    /// CHECK: address is checked
    pub switchboard_price_b: AccountInfo<'info>,
}

pub fn handler(ctx: Context<UpdateLpPrice>) -> Result<()> {
    let lp_price_acc = &mut ctx.accounts.lp_price_account;
    let vault = &ctx.accounts.vault_account;
    let supply = ctx.accounts.vault_lp_token_mint_pubkey.supply;

    let whirlpool = {
        let acc_data_slice: &[u8] = &ctx.accounts.whirlpool.try_borrow_data()?;
        Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?
    };

    let position = {
        let acc_data_slice: &[u8] = &ctx.accounts.position.try_borrow_data()?;
        Position::try_deserialize(&mut acc_data_slice.borrow())?
    };

    let price_a_agg = AggregatorAccountData::new(&ctx.accounts.switchboard_price_a)?;
    let price_b_agg = AggregatorAccountData::new(&ctx.accounts.switchboard_price_b)?;

    let (lamports_a, lamports_b) = {
        let vault_amount_a = ctx.accounts.vault_input_token_a_account.amount;
        let vault_amount_b = ctx.accounts.vault_input_token_b_account.amount;

        let past_liquidity = position.liquidity - vault.last_liquidity_increase;
        let liquidity_delta = convert_to_liquidity_delta(past_liquidity, false)
            .map_err(|_| error!(ErrorCode::WhirlpoolLiquidityTooHigh))?;

        let (amount_whirlpool_a, amount_whirlpool_b) =
            whirlpool::manager::liquidity_manager::calculate_liquidity_token_deltas(
                whirlpool.tick_current_index,
                whirlpool.sqrt_price,
                &position,
                liquidity_delta,
            )
            .map_err(|_| error!(ErrorCode::WhirlpoolLiquidityToDeltasOverflow))?;

        (
            vault_amount_a + amount_whirlpool_a,
            vault_amount_b + amount_whirlpool_b,
        )
    };

    let amount_a: f64 = (lamports_a as f64) / (lp_price_acc.token_a_mint_decimals as f64);
    let amount_b: f64 = (lamports_b as f64) / (lp_price_acc.token_b_mint_decimals as f64);

    let price_a: f64 = price_a_agg.get_result()?.try_into()?;
    let price_b: f64 = price_b_agg.get_result()?.try_into()?;

    let value = amount_a * price_a + amount_b * price_b;
    let value_per_token = value / (supply as f64);

    let std_value = {
        let std_price_a: f64 = price_a_agg
            .latest_confirmed_round
            .std_deviation
            .try_into()?;
        let std_price_b: f64 = price_b_agg
            .latest_confirmed_round
            .std_deviation
            .try_into()?;

        let partial_derivative_of_value_respect_price_a = amount_a;
        let partial_derivative_of_value_respect_price_b = amount_b;

        let std_value2 = (partial_derivative_of_value_respect_price_a * std_price_a).powf(2.0)
            + (partial_derivative_of_value_respect_price_b * std_price_b).powf(2.0);

        std_value2.sqrt()
    };

    Ok(())
}
