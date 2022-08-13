use crate::error::ErrorCode;
use crate::state::{
    Bumps, InitVaultAccountParams, MarketRewards, MarketRewardsInfo, VaultAccount,
    NUM_MARKET_REWARDS,
};
use crate::{FEE_SCALE, VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct MarketRewardsInfoInput {
    /// Pubkey of the mint output to swap the rewards for
    pub is_destination_token_a: bool,
    /// Id of market associated
    pub id: MarketRewards,
}

#[derive(Accounts)]
#[instruction(vault_id: u8)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub user_signer: Signer<'info>,

    #[account(owner = whirlpool::ID)]
    /// CHECK: owner and account data is checked
    pub whirlpool: AccountInfo<'info>,

    pub input_token_a_mint_address: Account<'info, Mint>,
    pub input_token_b_mint_address: Account<'info, Mint>,
    #[account(
        init,
        payer = user_signer,
        space = 8 + VaultAccount::SIZE,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_id][..], whirlpool.key().as_ref()],
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
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeVault>,
    vault_id: u8,
    fee: u64,
    market_rewards_input: Vec<MarketRewardsInfoInput>,
) -> Result<()> {
    // Ensure the whirlpool has the right account data
    let (token_mint_a, token_mint_b, reward_infos) = {
        use anchor_lang_for_whirlpool::AccountDeserialize;
        use std::borrow::Borrow;

        let acc_data_slice: &[u8] = &ctx.accounts.whirlpool.try_borrow_data()?;
        let pool =
            whirlpool::state::whirlpool::Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?;
        (pool.token_mint_a, pool.token_mint_b, pool.reward_infos)
    };

    require!(
        ctx.accounts.input_token_a_mint_address.key() == token_mint_a,
        ErrorCode::InvalidInputMint
    );
    require!(
        ctx.accounts.input_token_b_mint_address.key() == token_mint_b,
        ErrorCode::InvalidInputMint
    );
    // Fee can't be more than 100%
    require!(fee <= FEE_SCALE, ErrorCode::InvalidFee);

    let mut market_rewards_info: [MarketRewardsInfo; NUM_MARKET_REWARDS] = Default::default();
    market_rewards_input.iter().enumerate().for_each(|(i, x)| {
        market_rewards_info[i] = MarketRewardsInfo {
            rewards_mint: reward_infos[i].mint,
            is_destination_token_a: x.is_destination_token_a,
            id: x.id,
        }
    });

    ctx.accounts
        .vault_account
        .set_inner(VaultAccount::init(InitVaultAccountParams {
            vault_id,
            bumps: Bumps {
                vault: *ctx.bumps.get("vault_account").unwrap(),
                lp_token_mint: *ctx.bumps.get("vault_lp_token_mint_pubkey").unwrap(),
            },
            whirlpool_id: ctx.accounts.whirlpool.key(),
            input_token_a_mint_pubkey: ctx.accounts.input_token_a_mint_address.key(),
            input_token_b_mint_pubkey: ctx.accounts.input_token_b_mint_address.key(),
            fee,
            market_rewards_info,
        }));

    Ok(())
}
