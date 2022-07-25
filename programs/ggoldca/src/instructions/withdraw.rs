use crate::instructions::DepositWithdraw;
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
    msg!("0.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("0.B {}", ctx.accounts.vault_input_token_b_account.amount);
    msg!("0.L {}", ctx.accounts.position.liquidity()?);

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

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;
    msg!("1.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("1.B {}", ctx.accounts.vault_input_token_b_account.amount);
    msg!("1.L {}", ctx.accounts.position.liquidity()?);

    let user_a = ctx.accounts.user_token_a_account.amount;
    let user_b = ctx.accounts.user_token_b_account.amount;
    ctx.accounts.user_token_a_account.reload()?;
    ctx.accounts.user_token_b_account.reload()?;
    msg!("U.A {}", ctx.accounts.user_token_a_account.amount - user_a);
    msg!("U.B {}", ctx.accounts.user_token_b_account.amount - user_b);

    Ok(())
}
