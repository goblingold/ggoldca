//! NAZARE: Liquidity Management for Orca Whirlpools
use anchor_lang::prelude::*;
use error::ErrorCode;
use instructions::*;

pub mod error;
pub mod instructions;
pub mod interfaces;
pub mod macros;
pub mod math;
pub mod state;

declare_id!("NAZAREQQuCnkV8CpkGZaoB6ccmvikM8uRr4GKPWwmPT");

// DrrB1p8sxhwBZ3cXE8u5t2GxqEcTNuwAm7RcrQ8Yqjod
pub const ADMIN_PUBKEY: Pubkey = Pubkey::new_from_array([
    191, 17, 77, 109, 253, 243, 16, 188, 64, 67, 249, 18, 51, 62, 173, 81, 128, 208, 121, 29, 74,
    57, 94, 247, 114, 4, 114, 88, 209, 115, 147, 136,
]);

// 8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1
pub const TREASURY_PUBKEY: Pubkey = Pubkey::new_from_array([
    111, 222, 226, 197, 174, 64, 51, 181, 235, 205, 56, 138, 76, 105, 173, 158, 191, 43, 143, 141,
    91, 145, 78, 45, 130, 86, 102, 175, 146, 188, 82, 152,
]);

pub const VAULT_ACCOUNT_SEED: &[u8; 5] = b"vault";
pub const VAULT_LP_TOKEN_MINT_SEED: &[u8; 4] = b"mint";

pub const FEE_SCALE: u64 = 100;
pub const VAULT_VERSION: u8 = 1;

pub const IS_PAUSED: bool = false;

#[program]
pub mod ggoldca {

    use super::*;

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        id: u8,
        fee: u64,
        min_slots_for_reinvest: u64,
    ) -> Result<()> {
        instructions::initialize_vault::handler(ctx, id, fee, min_slots_for_reinvest)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn set_vault_pause_status(
        ctx: Context<SetVaultPauseStatus>,
        is_paused: bool,
    ) -> Result<()> {
        instructions::set_vault_pause_status::handler(ctx, is_paused)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    pub fn set_vault_ui_status(ctx: Context<SetVaultUiStatus>, is_active: bool) -> Result<()> {
        instructions::set_vault_ui_status::handler(ctx, is_active)
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
    pub fn set_min_slots_for_reinvest(
        ctx: Context<SetMinSlotsForReinvest>,
        min_slots: u64,
    ) -> Result<()> {
        instructions::set_min_slots_for_reinvest::handler(ctx, min_slots)
    }

    #[access_control(is_admin(ctx.accounts.user_signer.key))]
    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn rebalance(ctx: Context<Rebalance>) -> Result<()> {
        instructions::rebalance::handler(ctx)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn deposit(
        ctx: Context<DepositWithdraw>,
        lp_amount: u64,
        max_amount_a: u64,
        max_amount_b: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, lp_amount, max_amount_a, max_amount_b)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn withdraw(
        ctx: Context<DepositWithdraw>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_amount, min_amount_a, min_amount_b)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn collect_fees(ctx: Context<CollectFees>) -> Result<()> {
        instructions::collect_fees::handler(ctx)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn collect_rewards(ctx: Context<CollectRewards>, reward_index: u8) -> Result<()> {
        instructions::collect_rewards::handler(ctx, reward_index)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn swap_rewards<'info>(ctx: Context<'_, '_, '_, 'info, SwapRewards<'info>>) -> Result<()> {
        instructions::swap_rewards::handler(ctx)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
    pub fn transfer_rewards(ctx: Context<TransferRewards>) -> Result<()> {
        instructions::transfer_rewards::handler(ctx)
    }

    #[access_control(is_paused(ctx.accounts.vault_account.is_paused))]
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

/// Check if the smart contract is paused either globally or at the vault level
fn is_paused(is_vault_paused: bool) -> Result<()> {
    require!(!IS_PAUSED, ErrorCode::PausedSmartContract);
    require!(!is_vault_paused, ErrorCode::PausedVault);
    Ok(())
}
