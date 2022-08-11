use crate::error::ErrorCode;
use crate::interfaces::orca_swap_v2;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang_for_whirlpool::{
    context::CpiContext as CpiContextForWhirlpool, AccountDeserialize,
};
use anchor_spl::token::{Token, TokenAccount};
use std::borrow::Borrow;
use whirlpool::math::tick_math::{MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64};

#[event]
pub struct SwapEvent {
    pub mint_in: Pubkey,
    pub amount_in: u64,
    pub mint_out: Pubkey,
    pub amount_out: u64,
}

#[derive(Accounts)]
pub struct SwapRewards<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(
        mut,
        // TODO ensure this is a reward account. Other checks? Check mints from deserialized wirlpool?
        constraint = vault_rewards_token_account.mint != vault_account.input_token_a_mint_pubkey
                  && vault_rewards_token_account.mint != vault_account.input_token_b_mint_pubkey,
        associated_token::mint = vault_rewards_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_rewards_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_destination_token_account.mint == vault_account.input_token_a_mint_pubkey
                  || vault_destination_token_account.mint == vault_account.input_token_b_mint_pubkey,
        associated_token::mint = vault_destination_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,

    /// CHECK: address is checked
    pub swap_program: AccountInfo<'info>,
}

impl<'info> SwapRewards<'info> {
    fn swap_orca_ctx(
        &self,
        remaining: &[AccountInfo<'info>],
    ) -> CpiContext<'_, '_, '_, 'info, orca_swap_v2::Swap<'info>> {
        CpiContext::new(
            self.swap_program.to_account_info(),
            orca_swap_v2::Swap {
                token_program: self.token_program.to_account_info(),
                user_account: self.vault_account.to_account_info(),
                user_token_a_account: self.vault_rewards_token_account.to_account_info(),
                user_token_b_account: self.vault_destination_token_account.to_account_info(),
                amm_id: remaining[0].to_account_info(),
                amm_authority: remaining[1].to_account_info(),
                pool_token_a_account: remaining[2].to_account_info(),
                pool_token_b_account: remaining[3].to_account_info(),
                lp_token_mint: remaining[4].to_account_info(),
                fees_account: remaining[5].to_account_info(),
            },
        )
    }

    fn whirlpool_swap_ctx(
        &self,
        remaining: &[AccountInfo<'info>],
        rewards_acc_is_token_a: bool,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::Swap<'info>> {
        let (token_owner_account_a, token_owner_account_b) = if rewards_acc_is_token_a {
            (
                self.vault_rewards_token_account.to_account_info(),
                self.vault_destination_token_account.to_account_info(),
            )
        } else {
            (
                self.vault_destination_token_account.to_account_info(),
                self.vault_rewards_token_account.to_account_info(),
            )
        };

        CpiContextForWhirlpool::new(
            self.swap_program.to_account_info(),
            whirlpool::cpi::accounts::Swap {
                token_program: self.token_program.to_account_info(),
                token_authority: self.vault_account.to_account_info(),
                token_owner_account_a,
                token_owner_account_b,
                whirlpool: remaining[0].to_account_info(),
                token_vault_a: remaining[1].to_account_info(),
                token_vault_b: remaining[2].to_account_info(),
                tick_array_0: remaining[3].to_account_info(),
                tick_array_1: remaining[4].to_account_info(),
                tick_array_2: remaining[5].to_account_info(),
                oracle: remaining[6].to_account_info(),
            },
        )
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
    msg!("0.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!(
        "0.B {}",
        ctx.accounts.vault_destination_token_account.amount
    );

    let amount_before = ctx.accounts.vault_destination_token_account.amount;

    match ctx.accounts.swap_program.key() {
        id if id == orca_swap_v2::ID => swap_orca_cpi(&ctx),
        id if id == whirlpool::ID => swap_whirlpool_cpi(&ctx),
        _ => Err(ErrorCode::InvalidSwapProgramId.into()),
    }?;

    ctx.accounts.vault_rewards_token_account.reload()?;
    ctx.accounts.vault_destination_token_account.reload()?;

    let amount_after = ctx.accounts.vault_destination_token_account.amount;
    let amount_increase = amount_after.safe_sub(amount_before)?;

    let vault = &mut ctx.accounts.vault_account;
    if ctx.accounts.vault_destination_token_account.mint == vault.input_token_a_mint_pubkey {
        vault.earned_rewards_token_a = vault.earned_rewards_token_a.safe_add(amount_increase)?;
    } else {
        vault.earned_rewards_token_b = vault.earned_rewards_token_b.safe_add(amount_increase)?;
    }

    msg!("1.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!(
        "1.B {}",
        ctx.accounts.vault_destination_token_account.amount
    );

    emit!(SwapEvent {
        mint_in: ctx.accounts.vault_rewards_token_account.mint,
        amount_in: ctx.accounts.vault_rewards_token_account.amount,
        mint_out: ctx.accounts.vault_destination_token_account.mint,
        amount_out: amount_increase,
    });

    Ok(())
}

fn swap_orca_cpi<'info>(ctx: &Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
    require!(ctx.remaining_accounts.len() == 6, InvalidNumberOfAccounts);

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    orca_swap_v2::swap(
        ctx.accounts
            .swap_orca_ctx(ctx.remaining_accounts)
            .with_signer(signer),
        ctx.accounts.vault_rewards_token_account.amount,
        1,
    )?;

    Ok(())
}

fn swap_whirlpool_cpi<'info>(ctx: &Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
    require!(ctx.remaining_accounts.len() == 7, InvalidNumberOfAccounts);

    let rewards_acc_is_token_a = {
        let acc_data_slice: &[u8] = &ctx.remaining_accounts[0].try_borrow_data()?;
        let pool =
            whirlpool::state::whirlpool::Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?;

        ctx.accounts.vault_rewards_token_account.mint == pool.token_mint_a
    };

    let is_swap_from_a_to_b = rewards_acc_is_token_a;
    let sqrt_price_limit = if is_swap_from_a_to_b {
        MIN_SQRT_PRICE_X64
    } else {
        MAX_SQRT_PRICE_X64
    };

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::swap(
        ctx.accounts
            .whirlpool_swap_ctx(ctx.remaining_accounts, rewards_acc_is_token_a)
            .with_signer(signer),
        ctx.accounts.vault_rewards_token_account.amount,
        1,
        sqrt_price_limit,
        true,
        is_swap_from_a_to_b,
    )?;

    Ok(())
}
