use crate::error::ErrorCode;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::VaultAccount;
use crate::{FEE_SCALE, TREASURY_PUBKEY, VAULT_ACCOUNT_SEED};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use whirlpool::cpi::accounts::{CollectReward, UpdateFeesAndRewards};

#[derive(Accounts)]
pub struct CollectRewards<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(
        mut,
        associated_token::mint = vault_rewards_token_account.mint,
        associated_token::authority = vault_account,
    )]
    pub vault_rewards_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = treasury_rewards_token_account.mint,
        associated_token::authority = TREASURY_PUBKEY
    )]
    pub treasury_rewards_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub reward_vault: AccountInfo<'info>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(
        constraint = position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = vault_account.position_address_exists(position.position.key()) @ ErrorCode::PositionNotActive
    )]
    pub position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> CollectRewards<'info> {
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

    fn collect_rewards_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, CollectReward<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            CollectReward {
                whirlpool: self.position.whirlpool.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                reward_vault: self.reward_vault.to_account_info(),
                reward_owner_account: self.vault_rewards_token_account.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    fn transfer_from_vault_to_treasury_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.vault_rewards_token_account.to_account_info(),
                to: self.treasury_rewards_token_account.to_account_info(),
                authority: self.vault_account.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<CollectRewards>, reward_index: u8) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let amount_before = ctx.accounts.vault_rewards_token_account.amount;

    // ORCA doesn't allow to update the fees and rewards for a position with zero liquidity
    if ctx.accounts.position.liquidity()? > 0 {
        whirlpool::cpi::update_fees_and_rewards(ctx.accounts.update_fees_and_rewards_ctx())?;
    }
    whirlpool::cpi::collect_reward(
        ctx.accounts.collect_rewards_ctx().with_signer(signer),
        reward_index,
    )?;

    ctx.accounts.vault_rewards_token_account.reload()?;

    let amount_after = ctx.accounts.vault_rewards_token_account.amount;
    let amount_increase = amount_after.safe_sub(amount_before)?;

    if ctx.accounts.vault_account.fee > 0 {
        let treasury_fee =
            amount_increase.safe_mul_div_round_up(ctx.accounts.vault_account.fee, FEE_SCALE)?;

        require!(treasury_fee > 0, ErrorCode::NotEnoughRewards);

        token::transfer(
            ctx.accounts
                .transfer_from_vault_to_treasury_ctx()
                .with_signer(signer),
            treasury_fee,
        )?;
    }

    Ok(())
}
