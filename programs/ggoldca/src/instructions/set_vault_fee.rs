use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetVaultFee<'info> {
    #[account()]
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.key().as_ref()],
        bump
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
}

pub fn handler(ctx: Context<SetVaultFee>, fee: u64) -> Result<()> {
    ctx.accounts.vault_account.fee = fee;
    Ok(())
}
