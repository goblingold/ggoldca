use crate::state::{Bumps, InitVaultAccountParams, VaultAccount};
use crate::TREASURY_PUBKEY;
use crate::{VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub user_signer: Signer<'info>,
    pub input_token_a_mint_address: Account<'info, Mint>,
    pub input_token_b_mint_address: Account<'info, Mint>,
    #[account(
        init,
        payer = user_signer,
        space = 8 + VaultAccount::SIZE,
        seeds = [VAULT_ACCOUNT_SEED, input_token_a_mint_address.key().as_ref(), input_token_b_mint_address.key().as_ref()],
        bump
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,
    #[account(
        init,
        payer = user_signer,
        associated_token::mint = input_token_a_mint_address,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = user_signer,
        associated_token::mint = input_token_b_mint_address,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = user_signer,
        // TODO check decimals
        mint::decimals = input_token_a_mint_address.decimals,
        mint::authority = vault_account.key(),
        seeds = [VAULT_LP_TOKEN_MINT_SEED, vault_account.key().as_ref()],
        bump
    )]
    pub vault_lp_token_mint_pubkey: Account<'info, Mint>,
    #[account(
        init,
        payer = user_signer,
        associated_token::mint = vault_lp_token_mint_pubkey,
        associated_token::authority = dao_treasury_owner,
    )]
    pub dao_treasury_lp_token_account: Account<'info, TokenAccount>,
    #[account(constraint = dao_treasury_owner.key == &TREASURY_PUBKEY)]
    /// CHECKED: address is checked
    pub dao_treasury_owner: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializeVault>, bump_vault: u8, bump_lp: u8) -> Result<()> {
    ctx.accounts
        .vault_account
        .set_inner(VaultAccount::init(InitVaultAccountParams {
            bumps: Bumps {
                vault: bump_vault,
                lp_token_mint: bump_lp,
            },
            input_token_a_mint_pubkey: ctx.accounts.input_token_a_mint_address.key(),
            input_token_b_mint_pubkey: ctx.accounts.input_token_b_mint_address.key(),
            dao_treasury_lp_token_account: ctx.accounts.dao_treasury_lp_token_account.key(),
        }));

    Ok(())
}
