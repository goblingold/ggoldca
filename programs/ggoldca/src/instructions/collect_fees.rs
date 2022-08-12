use crate::error::ErrorCode;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::VaultAccount;
use crate::{FEE_PERCENTAGE, TREASURY_PUBKEY, VAULT_ACCOUNT_SEED};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
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

    whirlpool::cpi::update_fees_and_rewards(ctx.accounts.update_fees_and_rewards_ctx())?;
    whirlpool::cpi::collect_fees(ctx.accounts.collect_fees_ctx().with_signer(signer))?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    let amount_a_after = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b_after = ctx.accounts.vault_input_token_b_account.amount;

    let amount_a_increase = amount_a_after.safe_sub(amount_a_before)?;
    let amount_b_increase = amount_b_after.safe_sub(amount_b_before)?;

    if FEE_PERCENTAGE > 0 {
        let treasury_fee_a = amount_a_increase.safe_mul_div_round_up(FEE_PERCENTAGE, 100_u64)?;
        let treasury_fee_b = amount_b_increase.safe_mul_div_round_up(FEE_PERCENTAGE, 100_u64)?;

        require!(treasury_fee_a > 0, ErrorCode::NotEnoughFees);
        require!(treasury_fee_b > 0, ErrorCode::NotEnoughFees);

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

    Ok(())
}
