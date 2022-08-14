use crate::error::ErrorCode;
use crate::state::{MarketRewards, MarketRewardsInfo, VaultAccount};
use crate::{VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct MarketRewardsInfoInput {
    /// Id of market associated
    pub id: MarketRewards,
    /// Minimum number of lamports to receive during swap
    pub min_amount_out: u64,
}

#[derive(Accounts)]
pub struct SetMarketRewards<'info> {
    #[account()]
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(owner = whirlpool::ID)]
    /// CHECK: owner and account data is checked
    pub whirlpool: AccountInfo<'info>,
    pub rewards_mint: Account<'info, Mint>,
    pub destination_token_account: Account<'info, TokenAccount>,
}

pub fn handler(
    ctx: Context<SetMarketRewards>,
    market_rewards: MarketRewardsInfoInput,
) -> Result<()> {
    // Ensure the whirlpool has the right account data
    let reward_infos = {
        use anchor_lang_for_whirlpool::AccountDeserialize;
        use std::borrow::Borrow;

        let acc_data_slice: &[u8] = &ctx.accounts.whirlpool.try_borrow_data()?;
        let pool =
            whirlpool::state::whirlpool::Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?;
        pool.reward_infos
    };

    let index: usize = reward_infos
        .iter()
        .position(|ri| ri.mint == ctx.accounts.rewards_mint.key())
        .ok_or_else(|| error!(ErrorCode::InvalidRewardMint))?;

    let market = MarketRewardsInfo {
        id: market_rewards.id,
        rewards_mint: ctx.accounts.rewards_mint.key(),
        destination_token_account: ctx.accounts.destination_token_account.key(),
        min_amount_out: market_rewards.min_amount_out,
    };

    market.validate(
        ctx.accounts.destination_token_account.mint,
        ctx.accounts.vault_account.input_token_a_mint_pubkey,
        ctx.accounts.vault_account.input_token_b_mint_pubkey,
    )?;

    ctx.accounts.vault_account.market_rewards[index] = market;

    Ok(())
}
