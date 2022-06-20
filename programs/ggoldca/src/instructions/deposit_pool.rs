use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct DepositPool<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.input_token_a_mint_pubkey.as_ref(), vault_account.input_token_b_mint_pubkey.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(constraint = whirlpool_program_id.key == &whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    /// CHECK: whirlpool cpi
    pub whirlpool: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub position_authority: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub position: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub position_token_account: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_owner_account_a: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_owner_account_b: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub tick_array_lower: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub tick_array_upper: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> DepositPool<'info> {
    fn modify_liquidity_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>> {
        CpiContext::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: self.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                position_authority: self.position_authority.to_account_info(),
                position: self.position.to_account_info(),
                position_token_account: self.position_token_account.to_account_info(),
                token_owner_account_a: self.token_owner_account_a.to_account_info(),
                token_owner_account_b: self.token_owner_account_b.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_lower: self.tick_array_lower.to_account_info(),
                tick_array_upper: self.tick_array_upper.to_account_info(),
            },
        )
    }
}

pub fn handler(
    ctx: Context<DepositPool>,
    liquidity_amount: u128,
    max_amount_a: u64,
    max_amount_b: u64,
) -> ProgramResult {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::increase_liquidity(
        ctx.accounts.modify_liquidity_ctx().with_signer(signer),
        liquidity_amount,
        max_amount_a,
        max_amount_b,
    )?;

    Ok(())
}
