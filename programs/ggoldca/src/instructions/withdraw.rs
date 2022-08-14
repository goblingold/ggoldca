use crate::error::ErrorCode;
use crate::instructions::{DepositWithdraw, DepositWithdrawEvent};
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::math::safe_arithmetics::SafeMulDiv;
use anchor_lang::prelude::*;
use anchor_spl::token;

pub fn handler(
    ctx: Context<DepositWithdraw>,
    lp_amount: u64,
    mut min_amount_a: u64,
    mut min_amount_b: u64,
) -> Result<()> {
    require!(lp_amount > 0, ErrorCode::ZeroLpAmount);

    let amount_user_a_before = ctx.accounts.user_token_a_account.amount;
    let amount_user_b_before = ctx.accounts.user_token_b_account.amount;

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let supply = ctx.accounts.vault_lp_token_mint_pubkey.supply;

    let vault_amount_a = ctx.accounts.vault_input_token_a_account.amount;
    let vault_amount_b = ctx.accounts.vault_input_token_b_account.amount;

    if vault_amount_a > 0 {
        let amount_a = vault_amount_a.safe_mul_div(lp_amount, supply)?;
        min_amount_a = min_amount_a.saturating_sub(amount_a);

        token::transfer(
            ctx.accounts
                .transfer_token_a_from_vault_to_user_ctx()
                .with_signer(signer),
            amount_a,
        )?;
    }

    if vault_amount_b > 0 {
        let amount_b = vault_amount_b.safe_mul_div(lp_amount, supply)?;
        min_amount_b = min_amount_b.saturating_sub(amount_b);

        token::transfer(
            ctx.accounts
                .transfer_token_b_from_vault_to_user_ctx()
                .with_signer(signer),
            amount_b,
        )?;
    }

    let past_liquidity = ctx
        .accounts
        .position
        .liquidity()?
        .safe_sub(ctx.accounts.vault_account.last_liquidity_increase)?;

    let user_liquidity = past_liquidity.safe_mul_div(u128::from(lp_amount), u128::from(supply))?;

    whirlpool::cpi::decrease_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        user_liquidity,
        min_amount_a,
        min_amount_b,
    )?;

    token::burn(ctx.accounts.burn_user_lps_ctx(), lp_amount)?;

    ctx.accounts.user_token_a_account.reload()?;
    ctx.accounts.user_token_b_account.reload()?;

    let amount_user_a_after = ctx.accounts.user_token_a_account.amount;
    let amount_user_b_after = ctx.accounts.user_token_b_account.amount;

    let amount_user_a_diff = amount_user_a_after.safe_sub(amount_user_a_before)?;
    let amount_user_b_diff = amount_user_b_after.safe_sub(amount_user_b_before)?;

    emit!(DepositWithdrawEvent {
        vault_account: ctx.accounts.vault_account.key(),
        amount_a: amount_user_a_diff,
        amount_b: amount_user_b_diff,
        liquidity: user_liquidity
    });

    Ok(())
}
