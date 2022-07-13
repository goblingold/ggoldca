use crate::error::ErrorCode;
use crate::interface::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Token, TokenAccount};
use whirlpool::math::tick_math::{MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64};

#[derive(Accounts)]
pub struct Reinvest<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

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

    msg!("0.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("0.B {}", ctx.accounts.vault_input_token_b_account.amount);
    msg!("0.L {}", ctx.accounts.position.liquidity()?);
    msg!(
        "0.dL {}",
        ctx.accounts.vault_account.last_liquidity_increase
    );

    let amount_a_before = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_before = ctx.accounts.vault_input_token_b_account.amount;
    let liquidity_before = ctx.accounts.position.liquidity()?;

    // First try to deposit the max available amounts
    ctx.accounts.deposit_max_possible_liquidity_cpi(signer)?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;
    msg!("1.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("1.B {}", ctx.accounts.vault_input_token_b_account.amount);
    msg!("1.L {}", ctx.accounts.position.liquidity()?);

    // In a first approximation, swap half of the remaining tokens
    {
        let amount_a = ctx.accounts.vault_input_token_a_account.amount;
        let amount_b = ctx.accounts.vault_input_token_b_account.amount;

        let (amount_to_swap, sqrt_price_limit, is_swap_from_a_to_b) = if amount_a > 0 {
            (amount_a / 2, MIN_SQRT_PRICE_X64, true)
        } else {
            (amount_b / 2, MAX_SQRT_PRICE_X64, false)
        };

        whirlpool::cpi::swap(
            ctx.accounts.swap_ctx().with_signer(signer),
            amount_to_swap,      //amount
            0,                   //other_amount_threshold
            sqrt_price_limit,    //sqrt_price_limit
            true,                //amount_specified_is_input
            is_swap_from_a_to_b, //a_to_b
        )?;
    }

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;
    msg!("2.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("2.B {}", ctx.accounts.vault_input_token_b_account.amount);

    // Deposit a second time with the new swapped amounts
    ctx.accounts.deposit_max_possible_liquidity_cpi(signer)?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    let amount_a_after = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_after = ctx.accounts.vault_input_token_b_account.amount;
    let liquidity_after = ctx.accounts.position.liquidity()?;

    let amount_a_diff = amount_a_before.safe_sub(amount_a_after)?;
    let amount_b_diff = amount_b_before.safe_sub(amount_b_after)?;
    let liquidity_increase = liquidity_after.safe_sub(liquidity_before)?;

    let vault = &mut ctx.accounts.vault_account;
    vault.last_liquidity_increase = liquidity_increase;
    vault.acc_non_invested_fees_a = vault.acc_non_invested_fees_a.saturating_sub(amount_a_diff);
    vault.acc_non_invested_fees_b = vault.acc_non_invested_fees_b.saturating_sub(amount_b_diff);

    msg!("3.A {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("3.B {}", ctx.accounts.vault_input_token_b_account.amount);
    msg!("3.L {}", ctx.accounts.position.liquidity()?);
    msg!(
        "3.dL {}",
        ctx.accounts.vault_account.last_liquidity_increase
    );
    Ok(())
}
