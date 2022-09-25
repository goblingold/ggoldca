//! NAZARE: Liquidity Management for Orca Whirlpools
use crate::error::ErrorCode;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED, VAULT_VERSION};
use {
    anchor_lang::{prelude::*, solana_program},
    anchor_spl::token::Mint,
    anchor_spl::{associated_token::AssociatedToken, token::Token},
    mpl_token_metadata::{
        instruction::{create_metadata_accounts_v2, update_metadata_accounts_v2},
        state::DataV2,
    },
};

#[derive(Accounts)]
pub struct SetTokenMetadata<'info> {
    /// CHECK: Metadata key (pda of ['metadata', program id, mint id])
    #[account(mut,
      seeds = [b"metadata", mpl_token_metadata::id().as_ref(), vault_lp_token_mint_pubkey.key().as_ref()],
      seeds::program = mpl_token_metadata::id(),
      bump,
    )]
    pub metadata_account: AccountInfo<'info>,

    #[account(
      constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
      seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
      bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(
      mut,
      mint::authority = vault_account.key(),
      seeds = [VAULT_LP_TOKEN_MINT_SEED, vault_account.key().as_ref()],
      bump = vault_account.bumps.lp_token_mint
    )]
    pub vault_lp_token_mint_pubkey: Account<'info, Mint>,

    #[account(mut)]
    pub user_signer: Signer<'info>,

    #[account(address = mpl_token_metadata::id())]
    /// CHECK: I don't get why anchor complains about this, since we verify the address
    pub token_metadata_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<SetTokenMetadata>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
    first_time: bool,
) -> Result<()> {
    let ix = match first_time {
        true => create_metadata_accounts_v2(
            *ctx.accounts.token_metadata_program.key,
            *ctx.accounts.metadata_account.key,
            ctx.accounts.vault_lp_token_mint_pubkey.key(),
            ctx.accounts.vault_account.key(),
            ctx.accounts.user_signer.key(),
            ctx.accounts.user_signer.key(),
            token_name,
            token_symbol,
            token_uri,
            None,
            0,
            true,
            true,
            None,
            None,
        ),
        false => update_metadata_accounts_v2(
            *ctx.accounts.token_metadata_program.key,
            *ctx.accounts.metadata_account.key,
            ctx.accounts.user_signer.key(),
            Some(ctx.accounts.user_signer.key()),
            Some(DataV2 {
                name: token_name,
                symbol: token_symbol,
                uri: token_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            }),
            None,
            None,
        ),
    };

    let seeds = generate_seeds!(ctx.accounts.vault_account);

    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.metadata_account.clone(),
            ctx.accounts.vault_lp_token_mint_pubkey.to_account_info(),
            ctx.accounts.vault_account.to_account_info(),
            ctx.accounts.user_signer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        &[seeds],
    )?;

    Ok(())
}
