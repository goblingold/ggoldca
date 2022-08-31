use crate::error::ErrorCode;
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetVaultPauseStatus<'info> {
    #[account()]
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
}

pub fn handler(ctx: Context<SetVaultPauseStatus>, is_paused: bool) -> Result<()> {
    ctx.accounts.vault_account.is_paused = is_paused;
    Ok(())
}
