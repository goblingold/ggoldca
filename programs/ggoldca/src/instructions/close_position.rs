use crate::error::ErrorCode;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
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
        constraint = vault_account.position_address_exists(position.key()) @ ErrorCode::PositionNotActive
    )]
    /// CHECK: whirlpool cpi
    pub position: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position_mint: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position_token_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ClosePosition<'info> {
    fn close_position_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ClosePosition<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ClosePosition {
                position_authority: self.vault_account.to_account_info(),
                receiver: self.user_signer.to_account_info(),
                position: self.position.to_account_info(),
                position_mint: self.position_mint.to_account_info(),
                position_token_account: self.position_token_account.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<ClosePosition>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::close_position(ctx.accounts.close_position_ctx().with_signer(signer))?;

    Ok(())
}
