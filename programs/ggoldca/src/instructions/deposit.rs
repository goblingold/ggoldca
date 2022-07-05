use crate::error::ErrorCode;
use crate::interface::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::{SafeArithmetics, SafeMulDiv};
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Approve, Burn, Mint, MintTo, Revoke, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct DepositWithdraw<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.input_token_a_mint_pubkey.as_ref(), vault_account.input_token_b_mint_pubkey.as_ref()],
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
        token::authority = user_signer.key()
    )]
    pub user_lp_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::authority = user_signer.key()
    )]
    pub user_token_a_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::authority = user_signer.key()
    )]
    pub user_token_b_account: Account<'info, TokenAccount>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,
    pub position: PositionAccounts<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub wh_token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub wh_token_vault_b: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> DepositWithdraw<'info> {
    fn transfer_token_a_from_user_to_vault_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_user_to_vault_ctx(
            &self.user_token_a_account,
            &self.vault_input_token_a_account,
        )
    }

    fn transfer_token_b_from_user_to_vault_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_user_to_vault_ctx(
            &self.user_token_b_account,
            &self.vault_input_token_b_account,
        )
    }

    pub fn transfer_token_a_from_vault_to_user_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_vault_to_user_ctx(
            &self.vault_input_token_a_account,
            &self.user_token_a_account,
        )
    }

    pub fn transfer_token_b_from_vault_to_user_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        self._transfer_from_vault_to_user_ctx(
            &self.vault_input_token_b_account,
            &self.user_token_b_account,
        )
    }

    fn delegate_user_token_a_to_vault_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Approve<'info>> {
        self._delegate_user_to_vault_ctx(&self.user_token_a_account)
    }

    fn delegate_user_token_b_to_vault_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Approve<'info>> {
        self._delegate_user_to_vault_ctx(&self.user_token_b_account)
    }

    fn revoke_vault_from_user_token_a_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Revoke<'info>> {
        self._revoke_vault_from_user_ctx(&self.user_token_a_account)
    }

    fn revoke_vault_from_user_token_b_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Revoke<'info>> {
        self._revoke_vault_from_user_ctx(&self.user_token_b_account)
    }

    fn mint_lp_to_user_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.vault_lp_token_mint_pubkey.to_account_info(),
                to: self.user_lp_token_account.to_account_info(),
                authority: self.vault_account.to_account_info(),
            },
        )
    }

    pub fn burn_user_lps_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.vault_lp_token_mint_pubkey.to_account_info(),
                from: self.user_lp_token_account.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }

    pub fn modify_liquidity_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: self.position.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                token_owner_account_a: self.user_token_a_account.to_account_info(),
                token_owner_account_b: self.user_token_b_account.to_account_info(),
                token_vault_a: self.wh_token_vault_a.to_account_info(),
                token_vault_b: self.wh_token_vault_b.to_account_info(),
                tick_array_lower: self.position.tick_array_lower.to_account_info(),
                tick_array_upper: self.position.tick_array_upper.to_account_info(),
            },
        )
    }

    fn _transfer_from_user_to_vault_ctx(
        &self,
        user: &Account<'info, TokenAccount>,
        vault: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: user.to_account_info(),
                to: vault.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }

    fn _transfer_from_vault_to_user_ctx(
        &self,
        vault: &Account<'info, TokenAccount>,
        user: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: vault.to_account_info(),
                to: user.to_account_info(),
                authority: self.vault_account.to_account_info(),
            },
        )
    }

    fn _delegate_user_to_vault_ctx(
        &self,
        account: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, Approve<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Approve {
                to: account.to_account_info(),
                delegate: self.vault_account.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }

    fn _revoke_vault_from_user_ctx(
        &self,
        account: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, Revoke<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Revoke {
                source: account.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }
}

pub fn handler(
    ctx: Context<DepositWithdraw>,
    lp_amount: u64,
    mut max_amount_a: u64,
    mut max_amount_b: u64,
) -> Result<()> {
    let user_a = ctx.accounts.user_token_a_account.amount;
    let user_b = ctx.accounts.user_token_b_account.amount;

    let supply = ctx.accounts.vault_lp_token_mint_pubkey.supply;

    let liquidity = if supply > 0 {
        let vault_amount_a = ctx.accounts.vault_input_token_a_account.amount;
        let vault_amount_b = ctx.accounts.vault_input_token_b_account.amount;

        if vault_amount_a > 0 {
            let amount_a = vault_amount_a.safe_mul_div_round_up(lp_amount, supply)?;
            token::transfer(
                ctx.accounts.transfer_token_a_from_user_to_vault_ctx(),
                amount_a,
            )?;

            require!(amount_a < max_amount_a, ErrorCode::ExceededTokenMax);
            max_amount_a = max_amount_a.safe_sub(amount_a)?;
        }

        if vault_amount_b > 0 {
            let amount_b = vault_amount_b.safe_mul_div_round_up(lp_amount, supply)?;
            token::transfer(
                ctx.accounts.transfer_token_b_from_user_to_vault_ctx(),
                amount_b,
            )?;

            require!(amount_b < max_amount_b, ErrorCode::ExceededTokenMax);
            max_amount_b = max_amount_b.safe_sub(amount_b)?;
        }

        let current_liquidity = ctx.accounts.position.liquidity()?;
        current_liquidity.safe_mul_div_round_up(u128::from(lp_amount), u128::from(supply))?
    } else {
        u128::from(lp_amount)
    };

    token::approve(
        ctx.accounts.delegate_user_token_a_to_vault_ctx(),
        max_amount_a,
    )?;
    token::approve(
        ctx.accounts.delegate_user_token_b_to_vault_ctx(),
        max_amount_b,
    )?;

    msg!(
        "C {:?}",
        ctx.accounts
            .position
            .token_amounts_from_liquidity_round_up(u128::from(lp_amount))?
    );

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::increase_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        liquidity,
        max_amount_a,
        max_amount_b,
    )?;

    token::mint_to(
        ctx.accounts.mint_lp_to_user_ctx().with_signer(signer),
        lp_amount,
    )?;

    ctx.accounts.user_token_a_account.reload()?;
    ctx.accounts.user_token_b_account.reload()?;
    let liquidity_after = ctx.accounts.position.liquidity()?;
    msg!("A {}", user_a - ctx.accounts.user_token_a_account.amount);
    msg!("B {}", user_b - ctx.accounts.user_token_b_account.amount);
    msg!("L {}", liquidity_after);

    token::revoke(ctx.accounts.revoke_vault_from_user_token_a_ctx())?;
    token::revoke(ctx.accounts.revoke_vault_from_user_token_b_ctx())?;

    Ok(())
}
