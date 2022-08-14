use crate::error::ErrorCode;
use crate::state::VaultAccount;
use crate::{FEE_SCALE, VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetVaultFee<'info> {
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

pub fn handler(ctx: Context<SetVaultFee>, fee: u64) -> Result<()> {
    // Fee can't be more than 100%
    require!(fee <= FEE_SCALE, ErrorCode::InvalidFee);

    ctx.accounts.vault_account.fee = fee;
    Ok(())
}
