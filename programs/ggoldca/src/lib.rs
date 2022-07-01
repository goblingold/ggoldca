use anchor_lang::prelude::*;
use instructions::*;

mod error;
mod instructions;
mod macros;
mod math;
mod position;
mod state;

declare_id!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

// 8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1
const TREASURY_PUBKEY: Pubkey = Pubkey::new_from_array([
    111, 222, 226, 197, 174, 64, 51, 181, 235, 205, 56, 138, 76, 105, 173, 158, 191, 43, 143, 141,
    91, 145, 78, 45, 130, 86, 102, 175, 146, 188, 82, 152,
]);

const VAULT_ACCOUNT_SEED: &[u8; 5] = b"vault";
const VAULT_LP_TOKEN_MINT_SEED: &[u8; 4] = b"mint";

#[program]
pub mod ggoldca {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        bump_vault: u8,
        bump_lp: u8,
    ) -> Result<()> {
        instructions::initialize_vault::handler(ctx, bump_vault, bump_lp)
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        bump: u8,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<()> {
        instructions::open_position::handler(ctx, bump, tick_lower_index, tick_upper_index)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        liquidity_amount: u128,
        max_amount_a: u64,
        max_amount_b: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, liquidity_amount, max_amount_a, max_amount_b)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        liquidity_amount: u128,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, liquidity_amount, min_amount_a, min_amount_b)
    }

    pub fn collect_fees_and_rewards<'info>(
        ctx: Context<'_, '_, '_, 'info, CollectFeesAndRewards<'info>>,
    ) -> Result<()> {
        instructions::collect_fees_and_rewards::handler(ctx)
    }

    pub fn rebalance(ctx: Context<Rebalance>) -> Result<()> {
        instructions::rebalance::handler(ctx)
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount: u64,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool,
    ) -> Result<()> {
        instructions::swap::handler(
            ctx,
            amount,
            other_amount_threshold,
            sqrt_price_limit,
            amount_specified_is_input,
            a_to_b,
        )
    }
}
