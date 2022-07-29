use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");

pub fn swap<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Swap<'info>>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<()> {
    let data = spl_token_swap::instruction::Swap {
        amount_in,
        minimum_amount_out,
    };

    let ix = spl_token_swap::instruction::swap(
        &ID,
        &anchor_spl::token::ID,
        ctx.accounts.amm_id.key,
        ctx.accounts.amm_authority.key,
        ctx.accounts.user_account.key,
        ctx.accounts.user_token_a_account.key,
        ctx.accounts.pool_token_a_account.key,
        ctx.accounts.pool_token_b_account.key,
        ctx.accounts.user_token_b_account.key,
        ctx.accounts.lp_token_mint.key,
        ctx.accounts.fees_account.key,
        None,
        data,
    )?;

    invoke_signed(&ix, &ctx.to_account_infos(), ctx.signer_seeds)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Swap<'info> {
    /// CHECK: orca cpi
    pub token_program: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub user_account: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub user_token_a_account: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub user_token_b_account: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub amm_id: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub amm_authority: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub pool_token_a_account: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub pool_token_b_account: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub lp_token_mint: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub fees_account: AccountInfo<'info>,
}
