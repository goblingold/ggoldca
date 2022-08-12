use crate::error::ErrorCode;
use crate::state::{PositionInfo, VaultAccount, MAX_POSITIONS};
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub user_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_ACCOUNT_SEED, &[vault_account.vault_id][..], vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position: AccountInfo<'info>,
    #[account(signer, mut)]
    /// CHECK: whirlpool cpi
    pub position_mint: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub position_token_account: AccountInfo<'info>,
    /// CHECK: whirlpool cpi
    pub whirlpool: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> OpenPosition<'info> {
    fn open_position_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::OpenPosition<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::OpenPosition {
                funder: self.user_signer.to_account_info(),
                owner: self.vault_account.to_account_info(),
                position: self.position.to_account_info(),
                position_mint: self.position_mint.to_account_info(),
                position_token_account: self.position_token_account.to_account_info(),
                whirlpool: self.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                system_program: self.system_program.to_account_info(),
                rent: self.rent.to_account_info(),
                associated_token_program: self.associated_token_program.to_account_info(),
            },
        )
    }
}

pub fn handler(
    ctx: Context<OpenPosition>,
    bump: u8,
    tick_lower_index: i32,
    tick_upper_index: i32,
) -> Result<()> {
    whirlpool::cpi::open_position(
        ctx.accounts.open_position_ctx(),
        whirlpool::state::position::OpenPositionBumps {
            position_bump: bump,
        },
        tick_lower_index,
        tick_upper_index,
    )?;

    let vault = &mut ctx.accounts.vault_account;

    require!(
        !vault.position_exists(tick_lower_index, tick_upper_index),
        ErrorCode::PositionAlreadyOpened
    );

    require!(
        vault.positions.len() < MAX_POSITIONS,
        ErrorCode::PositionLimitReached
    );

    vault.positions.push(PositionInfo {
        pubkey: ctx.accounts.position.key(),
        lower_tick: tick_lower_index,
        upper_tick: tick_upper_index,
    });

    Ok(())
}
