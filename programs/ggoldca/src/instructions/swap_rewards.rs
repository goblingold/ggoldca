use crate::error::ErrorCode;
use crate::interfaces::orca_swap_v2;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::state::{MarketRewardsInfo, VaultAccount};
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

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum MarketRewards {
    OrcaV2,
    OrcaWhirlpool,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum InputTokens {
    TokenA,
    TokenB,
}

impl Default for MarketRewards {
    fn default() -> Self {
        MarketRewards::OrcaV2
    }
}
impl Default for InputTokens {
    fn default() -> Self {
        InputTokens::TokenA
    }
}

#[derive(Accounts)]
pub struct SwapRewards<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.vault_id][..], vault_account.whirlpool_id.as_ref()],
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
    let amount_to_swap = ctx.accounts.vault_rewards_token_account.amount;
    let amount_out_before = ctx.accounts.vault_destination_token_account.amount;

    let market_rewards: &MarketRewardsInfo = ctx
        .accounts
        .vault_account
        .market_rewards
        .iter()
        .find(|market| market.rewards_mint == ctx.accounts.vault_rewards_token_account.mint)
        .ok_or(ErrorCode::InvalidMarketRewards)?;

    if market_rewards.destination_mint_id == InputTokens::TokenA {
        require!(
            ctx.accounts.vault_account.input_token_a_mint_pubkey
                == ctx.accounts.vault_destination_token_account.mint,
            ErrorCode::InvalidSwap
        );
    } else {
        require!(
            ctx.accounts.vault_account.input_token_b_mint_pubkey
                == ctx.accounts.vault_destination_token_account.mint,
            ErrorCode::InvalidSwap
        );
    };

    match market_rewards.id {
        id if id == MarketRewards::OrcaV2 => swap_orca_cpi(&ctx, amount_to_swap),
        id if id == MarketRewards::OrcaWhirlpool => swap_whirlpool_cpi(&ctx, amount_to_swap),
        _ => Err(ErrorCode::InvalidSwap.into()),
    }?;

    ctx.accounts.vault_destination_token_account.reload()?;

    let amount_out_after = ctx.accounts.vault_destination_token_account.amount;
    let amount_out_increase = amount_out_after.safe_sub(amount_out_before)?;

    let vault = &mut ctx.accounts.vault_account;
    if ctx.accounts.vault_destination_token_account.mint == vault.input_token_a_mint_pubkey {
        vault.earned_rewards_token_a =
            vault.earned_rewards_token_a.safe_add(amount_out_increase)?;
    } else {
        vault.earned_rewards_token_b =
            vault.earned_rewards_token_b.safe_add(amount_out_increase)?;
    }

    emit!(SwapEvent {
        mint_in: ctx.accounts.vault_rewards_token_account.mint,
        amount_in: amount_to_swap,
        mint_out: ctx.accounts.vault_destination_token_account.mint,
        amount_out: amount_out_increase,
    });

    Ok(())
}

fn swap_orca_cpi<'info>(
    ctx: &Context<'_, '_, '_, 'info, SwapRewards<'info>>,
    amount_to_swap: u64,
) -> Result<()> {
    require!(ctx.remaining_accounts.len() == 6, InvalidNumberOfAccounts);

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    orca_swap_v2::swap(
        ctx.accounts
            .swap_orca_ctx(ctx.remaining_accounts)
            .with_signer(signer),
        amount_to_swap,
        1,
    )?;

    Ok(())
}

fn swap_whirlpool_cpi<'info>(
    ctx: &Context<'_, '_, '_, 'info, SwapRewards<'info>>,
    amount_to_swap: u64,
) -> Result<()> {
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
        amount_to_swap,
        1,
        sqrt_price_limit,
        true,
        is_swap_from_a_to_b,
    )?;

    Ok(())
}
