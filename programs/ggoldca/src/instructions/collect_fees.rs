use crate::error::ErrorCode;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::VaultAccount;
use crate::{FEE_SCALE, TREASURY_PUBKEY, VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use whirlpool::cpi::accounts::{CollectFees as WhCollectFees, UpdateFeesAndRewards};

#[event]
struct CollectFeesEvent {
    vault_account: Pubkey,
    total_fees_token_a: u64,
    total_fees_token_b: u64,
    treasury_fee_token_a: u64,
    treasury_fee_token_b: u64,
}

#[derive(Accounts)]
pub struct CollectFees<'info> {
    #[account(
        mut,
        constraint = vault_account.version == VAULT_VERSION @ ErrorCode::InvalidVaultVersion,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

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

    #[account(
        mut,
        associated_token::mint = vault_account.input_token_a_mint_pubkey,
        associated_token::authority = TREASURY_PUBKEY
    )]
    pub treasury_token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = TREASURY_PUBKEY
    )]
    pub treasury_token_b_account: Box<Account<'info, TokenAccount>>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    #[account(
        constraint = position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = vault_account.position_address_exists(position.position.key()) @ ErrorCode::PositionNonExistence
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

    fn transfer_token_a_from_vault_to_treasury_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_vault_to_treasury_ctx(
            &self.vault_input_token_a_account,
            &self.treasury_token_a_account,
        )
    }

    fn transfer_token_b_from_vault_to_treasury_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_vault_to_treasury_ctx(
            &self.vault_input_token_b_account,
            &self.treasury_token_b_account,
        )
    }

    fn _transfer_from_vault_to_treasury_ctx(
        &self,
        vault: &Account<'info, TokenAccount>,
        treasury: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: vault.to_account_info(),
                to: treasury.to_account_info(),
                authority: self.vault_account.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<CollectFees>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let amount_a_before = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_before = ctx.accounts.vault_input_token_b_account.amount;

    let has_zero_liquidity = ctx.accounts.position.liquidity()? == 0;

    // ORCA doesn't allow to update the fees and rewards for a position with zero liquidity
    if !has_zero_liquidity {
        whirlpool::cpi::update_fees_and_rewards(ctx.accounts.update_fees_and_rewards_ctx())?;
    }
    whirlpool::cpi::collect_fees(ctx.accounts.collect_fees_ctx().with_signer(signer))?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    let amount_a_after = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_after = ctx.accounts.vault_input_token_b_account.amount;

    let amount_a_increase = amount_a_after.safe_sub(amount_a_before)?;
    let amount_b_increase = amount_b_after.safe_sub(amount_b_before)?;

    let mut treasury_fee_a: u64 = 0;
    let mut treasury_fee_b: u64 = 0;

    if ctx.accounts.vault_account.fee > 0 {
        // amount increase > FEE SCALE in order to reduce the error produced by rounding
        // skip the check in order to be able to claim all pending rewards & close the position
        if !has_zero_liquidity {
            require!(amount_a_increase > FEE_SCALE, ErrorCode::NotEnoughFees);
            require!(amount_b_increase > FEE_SCALE, ErrorCode::NotEnoughFees);
        }

        treasury_fee_a =
            amount_a_increase.safe_mul_div_round_up(ctx.accounts.vault_account.fee, FEE_SCALE)?;
        treasury_fee_b =
            amount_b_increase.safe_mul_div_round_up(ctx.accounts.vault_account.fee, FEE_SCALE)?;

        token::transfer(
            ctx.accounts
                .transfer_token_a_from_vault_to_treasury_ctx()
                .with_signer(signer),
            treasury_fee_a,
        )?;
        token::transfer(
            ctx.accounts
                .transfer_token_b_from_vault_to_treasury_ctx()
                .with_signer(signer),
            treasury_fee_b,
        )?;
    }

    let vault = &mut ctx.accounts.vault_account;
    vault.earned_rewards_token_a = vault.earned_rewards_token_a.safe_add(amount_a_increase)?;
    vault.earned_rewards_token_b = vault.earned_rewards_token_b.safe_add(amount_b_increase)?;

    emit!(CollectFeesEvent {
        vault_account: ctx.accounts.vault_account.key(),
        total_fees_token_a: amount_a_increase,
        total_fees_token_b: amount_b_increase,
        treasury_fee_token_a: treasury_fee_a,
        treasury_fee_token_b: treasury_fee_b,
    });

    Ok(())
}
