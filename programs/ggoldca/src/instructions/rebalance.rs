use crate::interface::*;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
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

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    #[account(constraint = current_position.whirlpool.key == new_position.whirlpool.key)]
    pub current_position: PositionAccounts<'info>,
    pub new_position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Rebalance<'info> {
    fn modify_liquidity_ctx(
        &self,
        position: &PositionAccounts<'info>,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: position.whirlpool.to_account_info(),
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
}

pub fn handler(ctx: Context<Rebalance>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let liquidity = ctx.accounts.current_position.liquidity()?;

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
        .new_position
        .liquidity_from_token_amounts(amount_a, amount_b)?;

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
    msg!("2.L {}", ctx.accounts.new_position.liquidity()?);
    msg!("2.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("2.B {}", ctx.accounts.vault_input_token_b_account.amount);

    Ok(())
}
