use crate::instructions::DepositWithdraw;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeMulDiv;
use anchor_lang::prelude::*;
use anchor_spl::token;

pub fn handler(
    ctx: Context<DepositWithdraw>,
    lp_amount: u64,
    mut min_amount_a: u64,
    mut min_amount_b: u64,
) -> Result<()> {
    let user_a = ctx.accounts.user_token_a_account.amount;
    let user_b = ctx.accounts.user_token_b_account.amount;

    msg!("UA {}", user_a);
    msg!("UB {}", user_b);
    msg!("VA {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("VB {}", ctx.accounts.vault_input_token_b_account.amount);

    let supply = ctx.accounts.vault_lp_token_mint_pubkey.supply;
    msg!("lp {} supply {}", lp_amount, supply);

    let vault_amount_a = ctx.accounts.vault_input_token_a_account.amount;
    let vault_amount_b = ctx.accounts.vault_input_token_b_account.amount;

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    if vault_amount_a > 0 {
        let amount_a = vault_amount_a.safe_mul_div(lp_amount, supply)?;
        token::transfer(
            ctx.accounts
                .transfer_from_vault_to_user_ctx(
                    &ctx.accounts.vault_input_token_a_account,
                    &ctx.accounts.user_token_a_account,
                )
                .with_signer(signer),
            amount_a,
        )?;

        msg!("amount_a {}", amount_a);
        min_amount_a = min_amount_a.saturating_sub(amount_a);
    }

    if vault_amount_b > 0 {
        let amount_b = vault_amount_b.safe_mul_div(lp_amount, supply)?;
        token::transfer(
            ctx.accounts
                .transfer_from_vault_to_user_ctx(
                    &ctx.accounts.vault_input_token_b_account,
                    &ctx.accounts.user_token_b_account,
                )
                .with_signer(signer),
            amount_b,
        )?;

        msg!("amount_b {}", amount_b);
        min_amount_b = min_amount_b.saturating_sub(amount_b);
    }

    let user_liquidity_share = ctx
        .accounts
        .position
        .liquidity()?
        .safe_mul_div(u128::from(lp_amount), u128::from(supply))?;

    whirlpool::cpi::decrease_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        user_liquidity_share,
        min_amount_a,
        min_amount_b,
    )?;

    token::burn(ctx.accounts.burn_user_lps_ctx(), lp_amount)?;

    ctx.accounts.user_token_a_account.reload()?;
    ctx.accounts.user_token_b_account.reload()?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    msg!("UA {}", ctx.accounts.user_token_a_account.amount - user_a);
    msg!("UB {}", ctx.accounts.user_token_b_account.amount - user_b);
    msg!("LQ {}", ctx.accounts.position.liquidity()?);
    msg!("VA {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("VB {}", ctx.accounts.vault_input_token_b_account.amount);

    Ok(())
}
