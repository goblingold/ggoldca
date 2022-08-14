use crate::error::ErrorCode;
use crate::macros::generate_seeds;
use crate::state::{MarketRewards, MarketRewardsInfo, VaultAccount};
use crate::{VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

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
    /// CHECK: checked later with market_rewards
    pub destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

impl<'info> TransferRewards<'info> {
    fn transfer_from_vault_to_destination_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.vault_rewards_token_account.to_account_info(),
                to: self.destination_token_account.to_account_info(),
                authority: self.vault_account.to_account_info(),
            },
        )
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, TransferRewards<'info>>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let market_rewards: &MarketRewardsInfo = ctx
        .accounts
        .vault_account
        .market_rewards
        .iter()
        .find(|market| market.rewards_mint == ctx.accounts.vault_rewards_token_account.mint)
        .ok_or(ErrorCode::InvalidMarketRewards)?;

    require!(
        market_rewards.id == MarketRewards::Transfer,
        ErrorCode::InvalidDestinationAccount
    );
    require!(
        ctx.accounts.destination_token_account.key() == market_rewards.destination_token_account,
        ErrorCode::InvalidDestinationAccount
    );

    token::transfer(
        ctx.accounts
            .transfer_from_vault_to_destination_ctx()
            .with_signer(signer),
        ctx.accounts.vault_rewards_token_account.amount,
    )?;

    Ok(())
}
