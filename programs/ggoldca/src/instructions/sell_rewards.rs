use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{Token, TokenAccount};

// 9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP
const ORCA_SWAP_PROGRAM_V2_ID: Pubkey = Pubkey::new_from_array([
    126, 84, 119, 26, 87, 166, 241, 76, 169, 228, 2, 213, 74, 238, 69, 247, 55, 138, 202, 54, 92,
    123, 22, 154, 126, 200, 63, 81, 130, 178, 152, 240,
]);

#[derive(Accounts)]
pub struct SellRewards<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(
        mut,
        // TODO ensure this is a reward account. Other checks? Check mints from deserialized wirlpool?
        constraint = vault_rewards_token_account.mint != vault_account.input_token_a_mint_pubkey
                  && vault_rewards_token_account.mint != vault_account.input_token_b_mint_pubkey,
        associated_token::mint = vault_rewards_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_rewards_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_input_token_account.mint == vault_account.input_token_a_mint_pubkey
                  || vault_input_token_account.mint == vault_account.input_token_b_mint_pubkey,
        associated_token::mint = vault_input_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_account: Account<'info, TokenAccount>,

    #[account(address = ORCA_SWAP_PROGRAM_V2_ID)]
    /// CHECK: address is checked
    pub orca_program: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub pool: AccountInfo<'info>,
    /// CHECK: orca cpi
    pub pool_authority: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: orca cpi
    pub pool_source_token_account: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: orca cpi
    pub pool_destination_token_account: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: orca cpi
    pub pool_mint_account: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: orca cpi
    pub pool_fee_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SellRewards>) -> Result<()> {
    msg!("0.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!("0.B {}", ctx.accounts.vault_input_token_account.amount);

    let data = spl_token_swap::instruction::Swap {
        amount_in: ctx.accounts.vault_rewards_token_account.amount,
        minimum_amount_out: 0,
    };

    let ix = spl_token_swap::instruction::swap(
        &ORCA_SWAP_PROGRAM_V2_ID,
        &anchor_spl::token::ID,
        &ctx.accounts.pool.key(),
        &ctx.accounts.pool_authority.key(),
        &ctx.accounts.vault_account.key(),
        &ctx.accounts.vault_rewards_token_account.key(),
        &ctx.accounts.pool_source_token_account.key(),
        &ctx.accounts.pool_destination_token_account.key(),
        &ctx.accounts.vault_input_token_account.key(),
        &ctx.accounts.pool_mint_account.key(),
        &ctx.accounts.pool_fee_account.key(),
        None,
        data,
    )?;

    let accounts = [
        ctx.accounts.orca_program.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.pool.to_account_info(),
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.vault_account.to_account_info(),
        ctx.accounts.vault_rewards_token_account.to_account_info(),
        ctx.accounts.pool_source_token_account.to_account_info(),
        ctx.accounts
            .pool_destination_token_account
            .to_account_info(),
        ctx.accounts.vault_input_token_account.to_account_info(),
        ctx.accounts.pool_mint_account.to_account_info(),
        ctx.accounts.pool_fee_account.to_account_info(),
    ];

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    invoke_signed(&ix, &accounts, signer)?;

    ctx.accounts.vault_rewards_token_account.reload()?;
    ctx.accounts.vault_input_token_account.reload()?;
    msg!("1.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!("1.B {}", ctx.accounts.vault_input_token_account.amount);

    Ok(())
}
