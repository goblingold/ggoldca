use crate::error::ErrorCode;
use crate::interface::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeArithmetics;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Token, TokenAccount};
use whirlpool::cpi::accounts::{CollectFees as WhCollectFees, UpdateFeesAndRewards};

#[derive(Accounts)]
pub struct CollectFees<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = vault_account.input_token_a_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    #[account(
        constraint = position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = position.position.key() == vault_account.active_position_key() @ ErrorCode::PositionNotActive,
    )]
    pub position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> CollectFees<'info> {
    fn update_fees_and_rewards_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, UpdateFeesAndRewards<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            UpdateFeesAndRewards {
                whirlpool: self.position.whirlpool.to_account_info(),
                position: self.position.position.to_account_info(),
                tick_array_lower: self.position.tick_array_lower.to_account_info(),
                tick_array_upper: self.position.tick_array_upper.to_account_info(),
            },
        )
    }

    fn collect_fees_ctx(&self) -> CpiContextForWhirlpool<'_, '_, '_, 'info, WhCollectFees<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            WhCollectFees {
                whirlpool: self.position.whirlpool.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                token_owner_account_a: self.vault_input_token_a_account.to_account_info(),
                token_owner_account_b: self.vault_input_token_b_account.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<CollectFees>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let amount_a_before = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_before = ctx.accounts.vault_input_token_b_account.amount;

    whirlpool::cpi::update_fees_and_rewards(ctx.accounts.update_fees_and_rewards_ctx())?;
    whirlpool::cpi::collect_fees(ctx.accounts.collect_fees_ctx().with_signer(signer))?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    let amount_a_after = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_after = ctx.accounts.vault_input_token_b_account.amount;

    let amount_a_diff = amount_a_after.safe_sub(amount_a_before)?;
    let amount_b_diff = amount_b_after.safe_sub(amount_b_before)?;

    let vault = &mut ctx.accounts.vault_account;
    vault.acc_non_invested_fees_a = vault.acc_non_invested_fees_a.safe_add(amount_a_diff)?;
    vault.acc_non_invested_fees_b = vault.acc_non_invested_fees_b.safe_add(amount_b_diff)?;

    Ok(())
}
