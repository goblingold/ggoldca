use crate::error::ErrorCode;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang_for_whirlpool::{
    context::CpiContext as CpiContextForWhirlpool, AccountDeserialize,
};
use anchor_spl::token::{Token, TokenAccount};
use std::borrow::Borrow;
use whirlpool::math::tick_math::{MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64};

// 9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP
const ORCA_SWAP_PROGRAM_V2_ID: Pubkey = Pubkey::new_from_array([
    126, 84, 119, 26, 87, 166, 241, 76, 169, 228, 2, 213, 74, 238, 69, 247, 55, 138, 202, 54, 92,
    123, 22, 154, 126, 200, 63, 81, 130, 178, 152, 240,
]);

#[derive(Accounts)]
pub struct SwapRewards<'info> {
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
        constraint = vault_destination_token_account.mint == vault_account.input_token_a_mint_pubkey
                  || vault_destination_token_account.mint == vault_account.input_token_b_mint_pubkey,
        associated_token::mint = vault_destination_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,

    /// CHECK: address is checked
    pub swap_program: AccountInfo<'info>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
    msg!("0.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!(
        "0.B {}",
        ctx.accounts.vault_destination_token_account.amount
    );

    match ctx.accounts.swap_program.key() {
        ORCA_SWAP_PROGRAM_V2_ID => swap_orca_cpi(ctx.accounts, ctx.remaining_accounts),
        id if id == whirlpool::ID => swap_whirlpool_cpi(ctx.accounts, ctx.remaining_accounts),
        _ => Err(ErrorCode::InvalidSwapProgramId.into()),
    }?;

    ctx.accounts.vault_rewards_token_account.reload()?;
    ctx.accounts.vault_destination_token_account.reload()?;
    msg!("1.A {}", ctx.accounts.vault_rewards_token_account.amount);
    msg!(
        "1.B {}",
        ctx.accounts.vault_destination_token_account.amount
    );

    Ok(())
}

fn swap_orca_cpi<'info>(accs: &SwapRewards<'info>, remaining: &[AccountInfo<'info>]) -> Result<()> {
    require!(remaining.len() == 6, InvalidNumberOfAccounts);

    let data = spl_token_swap::instruction::Swap {
        amount_in: accs.vault_rewards_token_account.amount,
        minimum_amount_out: 0,
    };

    let ix = spl_token_swap::instruction::swap(
        &ORCA_SWAP_PROGRAM_V2_ID,
        &anchor_spl::token::ID,
        &remaining[0].key(),
        &remaining[1].key(),
        &accs.vault_account.key(),
        &accs.vault_rewards_token_account.key(),
        &remaining[2].key(),
        &remaining[3].key(),
        &accs.vault_destination_token_account.key(),
        &remaining[4].key(),
        &remaining[5].key(),
        None,
        data,
    )?;

    let mut accounts = vec![
        accs.swap_program.to_account_info(),
        accs.token_program.to_account_info(),
        accs.vault_account.to_account_info(),
        accs.vault_rewards_token_account.to_account_info(),
        accs.vault_destination_token_account.to_account_info(),
    ];
    accounts.extend_from_slice(remaining);

    let seeds = generate_seeds!(accs.vault_account);
    let signer = &[&seeds[..]];

    invoke_signed(&ix, &accounts, signer)?;

    Ok(())
}

fn swap_whirlpool_cpi<'info>(
    accs: &SwapRewards<'info>,
    remaining: &[AccountInfo<'info>],
) -> Result<()> {
    require!(remaining.len() == 7, InvalidNumberOfAccounts);

    let rewards_acc_is_token_a = {
        let acc_data_slice: &[u8] = &remaining[0].try_borrow_data()?;
        let pool =
            whirlpool::state::whirlpool::Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?;

        accs.vault_rewards_token_account.mint == pool.token_mint_a
    };

    let (token_owner_account_a, token_owner_account_b) = if rewards_acc_is_token_a {
        (
            accs.vault_rewards_token_account.to_account_info(),
            accs.vault_destination_token_account.to_account_info(),
        )
    } else {
        (
            accs.vault_destination_token_account.to_account_info(),
            accs.vault_rewards_token_account.to_account_info(),
        )
    };

    let cpi_ctx = CpiContextForWhirlpool::new(
        accs.swap_program.to_account_info(),
        whirlpool::cpi::accounts::Swap {
            token_program: accs.token_program.to_account_info(),
            token_authority: accs.vault_account.to_account_info(),
            token_owner_account_a,
            token_owner_account_b,
            whirlpool: remaining[0].to_account_info(),
            token_vault_a: remaining[1].to_account_info(),
            token_vault_b: remaining[2].to_account_info(),
            tick_array_0: remaining[3].to_account_info(),
            tick_array_1: remaining[4].to_account_info(),
            tick_array_2: remaining[5].to_account_info(),
            oracle: remaining[6].to_account_info(),
        },
    );

    let is_swap_from_a_to_b = rewards_acc_is_token_a;
    let sqrt_price_limit = if is_swap_from_a_to_b {
        MIN_SQRT_PRICE_X64
    } else {
        MAX_SQRT_PRICE_X64
    };

    let seeds = generate_seeds!(accs.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::swap(
        cpi_ctx.with_signer(signer),
        accs.vault_rewards_token_account.amount,
        0,
        sqrt_price_limit,
        true,
        is_swap_from_a_to_b,
    )?;

    Ok(())
}
