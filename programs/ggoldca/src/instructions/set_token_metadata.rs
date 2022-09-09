/* Copyright (C) 2022 Hedge Labs Inc. - All Rights Reserved
 * You may not distribute this code or share it with others
 * without the express permission of Hedge Labs Inc
 *
 * If you received a copy of this code from someone other
 * than Hedge Labs Inc, please reach out to Christopher
 * Coudron at coudron@hedge.so
 */
use {
    crate::id,
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
      address =  Pubkey::find_program_address(&[
          b"metadata", 
          mpl_token_metadata::id().as_ref(),
          mint.key().as_ref()
      ], &mpl_token_metadata::id()).0
  )]
    pub metadata_account: AccountInfo<'info>,

    // Do not check mint id as this will be set by the authority user
    // Only the admin can call this instruction
    #[account(mut)]
    //#[soteria(ignore)] This was set by the authority user. No need to validate.
    pub mint: Account<'info, Mint>, // mint of the token we are creating metadata for

    #[account(mut)]
    pub user_signer: Signer<'info>,

    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn exec(
    ctx: Context<SetTokenMetadata>,
    mint_pda_seed: String,
    token_name: String,
    token_symbol: String,
    token_uri: String,
    first_time: bool,
) -> Result<()> {
    let ix = match first_time {
        true => create_metadata_accounts_v2(
            *ctx.accounts.token_metadata_program.key,
            *ctx.accounts.metadata_account.key,
            ctx.accounts.mint.key(),
            ctx.accounts.mint.key(),
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

    // Derive token mint signing key
    let (_mint_address, mint_bump_seed) =
        Pubkey::find_program_address(&[mint_pda_seed.as_ref()], &id());
    let mint_signer_seeds: &[&[_]] = &[mint_pda_seed.as_ref(), &[mint_bump_seed]];

    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.metadata_account.clone(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.user_signer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        &[mint_signer_seeds],
    )?;

    Ok(())
}
