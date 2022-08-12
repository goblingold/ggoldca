use crate::error::ErrorCode;
use crate::state::{MarketRewardsInfo, VaultAccount};
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;

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
}

pub fn handler(
    ctx: Context<SetMarketRewards>,
    market_rewards_info: MarketRewardsInfo,
) -> Result<()> {
    Ok(())
}
