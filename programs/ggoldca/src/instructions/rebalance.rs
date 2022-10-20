use crate::error::ErrorCode;
use crate::interfaces::whirlpool_position::*;
use crate::macros::generate_seeds;
use crate::math::safe_arithmetics::SafeMulDiv;
use crate::state::VaultAccount;
use crate::{VAULT_ACCOUNT_SEED, VAULT_VERSION};
use anchor_lang::prelude::*;
use anchor_lang::{
    solana_program::{pubkey::Pubkey, sysvar},
    InstructionData,
};
use anchor_lang_for_whirlpool::context::CpiContext as CpiContextForWhirlpool;
use anchor_spl::token::{Token, TokenAccount};

#[event]
struct RebalanceEvent {
    vault_account: Pubkey,
    old_liquidity: u128,
    new_liquidity: u128,
}

#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub user_signer: Signer<'info>,
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
    pub vault_input_token_a_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = vault_account.input_token_b_mint_pubkey,
        associated_token::authority = vault_account,
    )]
    pub vault_input_token_b_account: Account<'info, TokenAccount>,

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
        constraint = current_position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = current_position.position.key() == vault_account.active_position_key() @ ErrorCode::PositionNotActive,
    )]
    pub current_position: PositionAccounts<'info>,
    #[account(
        constraint = new_position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = vault_account.position_address_exists(new_position.position.key()) @ ErrorCode::PositionNotActive,
        constraint = new_position.position.key() != current_position.position.key() @ ErrorCode::RebalanceIntoActivePosition
    )]
    pub new_position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: address is checked
    pub instructions_acc: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
    fn modify_liquidity_ctx(
        &self,
        position: &PositionAccounts<'info>,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, whirlpool::cpi::accounts::ModifyLiquidity<'info>>
    {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            whirlpool::cpi::accounts::ModifyLiquidity {
                whirlpool: position.whirlpool.to_account_info(),
                token_program: self.token_program.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: position.position.to_account_info(),
                position_token_account: position.position_token_account.to_account_info(),
                token_owner_account_a: self.vault_input_token_a_account.to_account_info(),
                token_owner_account_b: self.vault_input_token_b_account.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                tick_array_lower: position.tick_array_lower.to_account_info(),
                tick_array_upper: position.tick_array_upper.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<Rebalance>) -> Result<()> {
    require!(
        is_next_ix_reinvest(&ctx.accounts.instructions_acc)?,
        ErrorCode::MissingReinvest
    );

    // Allows a reinvest regardless of when the last one occurred
    ctx.accounts.vault_account.last_reinvestment_slot = 0;

    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    let init_liquidity = ctx.accounts.current_position.liquidity()?;

    whirlpool::cpi::decrease_liquidity(
        ctx.accounts
            .modify_liquidity_ctx(&ctx.accounts.current_position)
            .with_signer(signer),
        init_liquidity,
        0,
        0,
    )?;

    ctx.accounts.vault_input_token_a_account.reload()?;
    ctx.accounts.vault_input_token_b_account.reload()?;

    let amount_a = ctx.accounts.vault_input_token_a_account.amount;
    let amount_b = ctx.accounts.vault_input_token_b_account.amount;

    let new_liquidity = ctx
        .accounts
        .new_position
        .liquidity_from_token_amounts(amount_a, amount_b)?;

    whirlpool::cpi::increase_liquidity(
        ctx.accounts
            .modify_liquidity_ctx(&ctx.accounts.new_position)
            .with_signer(signer),
        new_liquidity,
        amount_a,
        amount_b,
    )?;

    let vault = &mut ctx.accounts.vault_account;

    let proportional_liquidity_increase = vault
        .last_liquidity_increase
        .safe_mul_div_round_up(new_liquidity, init_liquidity)?;

    vault.last_liquidity_increase = proportional_liquidity_increase;
    vault.update_active_position(ctx.accounts.new_position.position.key());

    emit!(RebalanceEvent {
        vault_account: ctx.accounts.vault_account.key(),
        old_liquidity: init_liquidity,
        new_liquidity,
    });

    Ok(())
}

fn is_next_ix_reinvest(instructions_acc: &AccountInfo) -> Result<bool> {
    let next_sighash: [u8; 8] = {
        let next_ix = sysvar::instructions::get_instruction_relative(1, instructions_acc)
            .map_err(|_| error!(ErrorCode::MissingIx))?;
        require!(next_ix.data.len() >= 8, ErrorCode::InvalidIxData);
        next_ix.data[..8].try_into().unwrap()
    };

    let reinvest_sighash = crate::instruction::Reinvest {}.data();

    Ok(reinvest_sighash == next_sighash)
}
