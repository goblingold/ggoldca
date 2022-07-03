use crate::error::ErrorCode;
use crate::interface::*;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Approve, Revoke, Token, TokenAccount};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.input_token_a_mint_pubkey.as_ref(), vault_account.input_token_b_mint_pubkey.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(constraint = whirlpool_program_id.key == &whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(mut)]
    pub token_owner_account_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_owner_account_b: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    pub position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    fn delegate_user_to_vault_ctx(
        &self,
        account: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Approve<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Approve {
                to: account,
                delegate: self.vault_account.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }

    fn revoke_vault_ctx(
        &self,
        account: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Revoke<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Revoke {
                source: account,
                authority: self.user_signer.to_account_info(),
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
                token_owner_account_a: self.token_owner_account_a.to_account_info(),
                token_owner_account_b: self.token_owner_account_b.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_lower: self.position.tick_array_lower.to_account_info(),
                tick_array_upper: self.position.tick_array_upper.to_account_info(),
            },
        )
    }
}

pub fn handler(
    ctx: Context<Deposit>,
    liquidity_amount: u128,
    max_amount_a: u64,
    max_amount_b: u64,
) -> Result<()> {
    token::approve(
        ctx.accounts
            .delegate_user_to_vault_ctx(ctx.accounts.token_owner_account_a.to_account_info()),
        max_amount_a,
    )?;

    token::approve(
        ctx.accounts
            .delegate_user_to_vault_ctx(ctx.accounts.token_owner_account_b.to_account_info()),
        max_amount_b,
    )?;

    let amount_a = ctx.accounts.token_owner_account_a.amount;
    let amount_b = ctx.accounts.token_owner_account_b.amount;

    let liquidity_before = ctx.accounts.position.liquidity()?;

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::increase_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        liquidity_amount,
        max_amount_a,
        max_amount_b,
    )?;

    let liquidity_after = ctx.accounts.position.liquidity()?;

    ctx.accounts.token_owner_account_a.reload()?;
    ctx.accounts.token_owner_account_b.reload()?;
    msg!("A {}", amount_a - ctx.accounts.token_owner_account_a.amount);
    msg!("B {}", amount_b - ctx.accounts.token_owner_account_b.amount);
    msg!("L {}", liquidity_after);

    let _user_liquidity = liquidity_after
        .checked_sub(liquidity_before)
        .ok_or_else(|| error!(ErrorCode::MathOverflow))?;

    token::revoke(
        ctx.accounts
            .revoke_vault_ctx(ctx.accounts.token_owner_account_a.to_account_info()),
    )?;

    token::revoke(
        ctx.accounts
            .revoke_vault_ctx(ctx.accounts.token_owner_account_b.to_account_info()),
    )?;

    Ok(())
}
