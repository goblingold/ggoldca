use crate::error::ErrorCode;
use crate::interfaces::orca_swap_v2;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::state::{MarketRewards, MarketRewardsInfo, VaultAccount};
use crate::{VAULT_ACCOUNT_SEED, VAULT_VERSION};
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
pub struct TransferRewards<'info> {
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(
        mut,
        constraint = vault_account.market_rewards.iter().any(|info| info.rewards_mint == vault_rewards_token_account.mint),
        associated_token::mint = vault_rewards_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_rewards_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, TransferRewards<'info>>) -> Result<()> {

    let market_rewards: &MarketRewardsInfo = ctx
        .accounts
        .vault_account
        .market_rewards
        .iter()
        .find(|market| market.rewards_mint == ctx.accounts.vault_rewards_token_account.mint)
        .ok_or(ErrorCode::InvalidMarketRewards)?;

    require!(market_rewards.id == MarketRewards::TransferRewards);
    require!(ctx.accounts.destination_token_account == market_rewards.destination_token_account, ErrorCode::InvalidDestinationAccount);
    


}

