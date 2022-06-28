use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct Swap<'info> {
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
    pub token_authority: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub whirlpool: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_owner_account_a: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_owner_account_b: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub tick_array_0: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub tick_array_1: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub tick_array_2: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub oracle: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    fn swap_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::Swap<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::Swap {
                token_program: self.token_program.to_account_info(),
                token_authority: self.token_authority.to_account_info(),
                whirlpool: self.whirlpool.to_account_info(),
                token_owner_account_a: self.token_owner_account_a.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_owner_account_b: self.token_owner_account_b.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_0: self.tick_array_0.to_account_info(),
                tick_array_1: self.tick_array_1.to_account_info(),
                tick_array_2: self.tick_array_2.to_account_info(),
                oracle: self.oracle.to_account_info(),
            },
        )
    }
}

pub fn handler(
    ctx: Context<Swap>,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::swap(
        ctx.accounts.swap_ctx().with_signer(signer),
        amount,
        other_amount_threshold,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
    )?;

    Ok(())
}
