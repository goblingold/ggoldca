use anchor_lang::prelude::*;
use instructions::*;

mod instructions;
mod macros;
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

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> ProgramResult {
        instructions::initialize_vault::handler(ctx)
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        bumps: whirlpool::state::position::OpenPositionBumps,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> ProgramResult {
        instructions::open_position::handler(ctx, bumps, tick_lower_index, tick_upper_index)
    }

    pub fn deposit_pool(
        ctx: Context<DepositPool>,
        liquidity_amount: u128,
        max_amount_a: u64,
        max_amount_b: u64,
    ) -> ProgramResult {
        instructions::deposit_pool::handler(ctx, liquidity_amount, max_amount_a, max_amount_b)
    }

    pub fn withdraw_pool(
        ctx: Context<DepositPool>,
        liquidity_amount: u128,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> ProgramResult {
        instructions::deposit_pool::handler(ctx, liquidity_amount, min_amount_a, min_amount_b)
    }
}
