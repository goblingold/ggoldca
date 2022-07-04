use crate::interface::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeMulDiv;
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_LP_TOKEN_MINT_SEED};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    #[account(constraint = whirlpool_program_id.key == &whirlpool::ID)]
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

impl<'info> Withdraw<'info> {
    fn transfer_from_vault_to_user_ctx(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from,
                to,
                authority: self.vault_account.to_account_info(),
            },
        )
    }

    fn burn_user_lps_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.vault_lp_token_mint_pubkey.to_account_info(),
                from: self.user_lp_token_account.to_account_info(),
                authority: self.user_signer.to_account_info(),
            },
        )
    }

    fn modify_liquidity_ctx(
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
}

pub fn handler(
    ctx: Context<Withdraw>,
    lp_amount: u64,
    mut min_amount_a: u64,
    mut min_amount_b: u64,
) -> Result<()> {
    let user_a = ctx.accounts.user_token_a_account.amount;
    let user_b = ctx.accounts.user_token_b_account.amount;

    msg!("UA {}", user_a);
    msg!("UB {}", user_b);
    msg!("VA {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("VB {}", ctx.accounts.vault_input_token_b_account.amount);

    let supply = ctx.accounts.vault_lp_token_mint_pubkey.supply;
    msg!("lp {} supply {}", lp_amount, supply);

    let vault_amount_a = ctx.accounts.vault_input_token_a_account.amount;
    let vault_amount_b = ctx.accounts.vault_input_token_b_account.amount;

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    if vault_amount_a > 0 {
        let amount_a = vault_amount_a.safe_mul_div(lp_amount, supply)?;
        token::transfer(
            ctx.accounts.transfer_from_vault_to_user_ctx(
                ctx.accounts.vault_input_token_a_account.to_account_info(),
                ctx.accounts.user_token_a_account.to_account_info(),
            ).with_signer(signer),
            amount_a,
        )?;

        msg!("amount_a {}", amount_a);
        min_amount_a = min_amount_a.saturating_sub(amount_a);
    }

    if vault_amount_b > 0 {
        let amount_b = vault_amount_b.safe_mul_div(lp_amount, supply)?;
        token::transfer(
            ctx.accounts.transfer_from_vault_to_user_ctx(
                ctx.accounts.vault_input_token_b_account.to_account_info(),
                ctx.accounts.user_token_b_account.to_account_info(),
            ).with_signer(signer),
            amount_b,
        )?;

        msg!("amount_b {}", amount_b);
        min_amount_b = min_amount_b.saturating_sub(amount_b);
    }

    let user_liquidity_share = ctx
        .accounts
        .position
        .liquidity()?
        .safe_mul_div(u128::from(lp_amount), u128::from(supply))?;


    whirlpool::cpi::decrease_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        user_liquidity_share,
        min_amount_a,
        min_amount_b,
    )?;

    token::burn(ctx.accounts.burn_user_lps_ctx(), lp_amount)?;

    ctx.accounts.user_token_a_account.reload()?;
    ctx.accounts.user_token_b_account.reload()?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    msg!("UA {}", ctx.accounts.user_token_a_account.amount - user_a);
    msg!("UB {}", ctx.accounts.user_token_b_account.amount - user_b);
    msg!("LQ {}", ctx.accounts.position.liquidity()?);
    msg!("VA {}", ctx.accounts.vault_input_token_a_account.amount);
    msg!("VB {}", ctx.accounts.vault_input_token_b_account.amount);

    Ok(())
}
