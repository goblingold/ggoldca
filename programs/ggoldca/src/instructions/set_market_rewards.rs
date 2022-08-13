use crate::error::ErrorCode;
use crate::state::{MarketRewardsInfo, VaultAccount};
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use super::swap_rewards::MarketRewards;

#[derive(Accounts)]
pub struct SetMarketRewards<'info> {
    #[account()]
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.key().as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(owner = whirlpool::ID)]
    /// CHECK: owner and account data is checked
    pub whirlpool: AccountInfo<'info>,
    pub rewards_mint: Account<'info, Mint>,
}

pub fn handler(
    ctx: Context<SetMarketRewards>,
    is_destination_token_a: bool,
    id: MarketRewards,
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

    ctx.accounts.vault_account.market_rewards[index] = MarketRewardsInfo {
        rewards_mint: ctx.accounts.rewards_mint.key(),
        is_destination_token_a,
        id,
    };

    Ok(())
}
