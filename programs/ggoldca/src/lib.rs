//! NAZARE: Liquidity Management for Orca Whirlpools
use anchor_lang::prelude::*;
use error::ErrorCode;
use instructions::*;

mod error;
mod instructions;
mod interfaces;
mod macros;
mod math;
mod state;

declare_id!("Nazareen6k6rAFXKKZrBj5PiehJsohQ8gwGFHJT77sa");

// HmSCy7wPWYRtydb6tM6zimoquTftjCJXscoREJ5WVHWH
const ADMIN_PUBKEY: Pubkey = Pubkey::new_from_array([
    249, 29, 12, 29, 3, 82, 20, 111, 148, 214, 220, 129, 139, 165, 228, 242, 175, 16, 205, 158, 59,
    56, 33, 55, 10, 244, 184, 213, 238, 120, 89, 82,
]);

// HtRAgo77BEsDBpKhebPtgfMgphVbe1Nyh1D2ZuVdAgUx
const TREASURY_PUBKEY: Pubkey = Pubkey::new_from_array([
    250, 230, 240, 19, 182, 177, 177, 53, 180, 11, 57, 236, 246, 189, 25, 184, 26, 255, 21, 203,
    103, 197, 254, 15, 15, 235, 161, 229, 171, 178, 211, 249,
]);

const VAULT_ACCOUNT_SEED: &[u8; 5] = b"vault";
const VAULT_LP_TOKEN_MINT_SEED: &[u8; 4] = b"mint";

const FEE_SCALE: u64 = 100;

const VAULT_VERSION: u8 = 1;

#[program]
pub mod ggoldca {

    use super::*;

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn initialize_vault(ctx: Context<InitializeVault>, id: u8, fee: u64) -> Result<()> {
        instructions::initialize_vault::handler(ctx, id, fee)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn open_position(
        ctx: Context<OpenPosition>,
        bump: u8,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<()> {
        instructions::open_position::handler(ctx, bump, tick_lower_index, tick_upper_index)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        instructions::close_position::handler(ctx)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn set_market_rewards(
        ctx: Context<SetMarketRewards>,
        market_rewards: MarketRewardsInfoInput,
    ) -> Result<()> {
        instructions::set_market_rewards::handler(ctx, market_rewards)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn set_vault_fee(ctx: Context<SetVaultFee>, fee: u64) -> Result<()> {
        instructions::set_vault_fee::handler(ctx, fee)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn rebalance(ctx: Context<Rebalance>) -> Result<()> {
        instructions::rebalance::handler(ctx)
    }

    pub fn deposit(
        ctx: Context<DepositWithdraw>,
        lp_amount: u64,
        max_amount_a: u64,
        max_amount_b: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, lp_amount, max_amount_a, max_amount_b)
    }

    pub fn withdraw(
        ctx: Context<DepositWithdraw>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_amount, min_amount_a, min_amount_b)
    }

    pub fn collect_fees(ctx: Context<CollectFees>) -> Result<()> {
        instructions::collect_fees::handler(ctx)
    }

    pub fn collect_rewards(ctx: Context<CollectRewards>, reward_index: u8) -> Result<()> {
        instructions::collect_rewards::handler(ctx, reward_index)
    }

    pub fn swap_rewards<'info>(ctx: Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
        instructions::swap_rewards::handler(ctx)
    }

    pub fn transfer_rewards(ctx: Context<TransferRewards>) -> Result<()> {
        instructions::transfer_rewards::handler(ctx)
    }

    pub fn reinvest(ctx: Context<Reinvest>) -> Result<()> {
        instructions::reinvest::handler(ctx)
    }
}

/// Check if target key is authorized
fn is_admin(key: &Pubkey) -> Result<()> {
    #[cfg(not(feature = "test"))]
    require!(key == &ADMIN_PUBKEY, ErrorCode::UnauthorizedUser);
    Ok(())
}
