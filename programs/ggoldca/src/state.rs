use crate::error::ErrorCode;
use anchor_lang::prelude::*;

use crate::VAULT_VERSION;

/// Number of simultaneous positions allowed
pub const MAX_POSITIONS: usize = 3;

/// Number of whirlpool rewards (from whirlpool::state::whirlpool::NUM_REWARDS)
pub const WHIRLPOOL_NUM_REWARDS: usize = 3;

/// Additional padding (8 * bytes)
const PADDING_AS_U64: usize = 10;

/// Strategy vault account
#[account]
#[derive(Default, Debug)]
pub struct VaultAccount {
    /// Vault version
    pub version: u8,
    /// Vault number for a given whirlpool
    pub id: u8,

    /// PDA bump seeds
    pub bumps: Bumps,

    /// Whirlpool pubkey
    pub whirlpool_id: Pubkey,
    /// Pool input token_a mint address
    pub input_token_a_mint_pubkey: Pubkey,
    /// Pool input token_b mint address
    pub input_token_b_mint_pubkey: Pubkey,

    /// Last reinvestment liquidity increase
    pub last_liquidity_increase: u128,

    /// Fee percentage using FEE_SCALE. Fee applied on earnings
    pub fee: u64,

    /// Total rewards earned by the vault
    pub earned_rewards_token_a: u64,
    pub earned_rewards_token_b: u64,

    /// The market where to sell the rewards
    pub market_rewards: [MarketRewardsInfo; WHIRLPOOL_NUM_REWARDS],

    /// Additional padding
    pub _padding: [u64; PADDING_AS_U64],

    /// Information about the opened positions (max = MAX_POSITIONS)
    pub positions: Vec<PositionInfo>,
}

impl VaultAccount {
    pub const SIZE: usize = 1
        + 1
        + Bumps::SIZE
        + 32
        + 32
        + 32
        + 16
        + 8
        + 8
        + 8
        + WHIRLPOOL_NUM_REWARDS * MarketRewardsInfo::SIZE
        + 8 * PADDING_AS_U64
        + 4
        + MAX_POSITIONS * PositionInfo::SIZE;

    /// Create a new vault
    pub fn new(params: VaultAccountParams) -> Self {
        Self {
            version: VAULT_VERSION,
            id: params.id,
            bumps: params.bumps,
            whirlpool_id: params.whirlpool_id,
            input_token_a_mint_pubkey: params.input_token_a_mint_pubkey,
            input_token_b_mint_pubkey: params.input_token_b_mint_pubkey,
            fee: params.fee,
            ..Self::default()
        }
    }

    /// Check the existence of a position
    pub fn position_exists(&self, tick_lower_index: i32, tick_upper_index: i32) -> bool {
        self.positions
            .iter()
            .any(|pos| pos.lower_tick == tick_lower_index && pos.upper_tick == tick_upper_index)
    }

    /// Check if the given pubkey is a valid position
    pub fn position_address_exists(&self, key: Pubkey) -> bool {
        self.positions.iter().any(|pos| pos.pubkey == key)
    }

    /// Return the current active position pubkey
    pub fn active_position_key(&self) -> Pubkey {
        self.positions[0].pubkey
    }

    /// Update the current active position
    pub fn update_active_position(&mut self, key: Pubkey) {
        let new_position_indx = self
            .positions
            .iter()
            .position(|p| p.pubkey == key)
            // this cannot fail, existence of positions checked in constraints
            .unwrap();
        self.positions.swap(0, new_position_indx)
    }
}

/// Create a new vault
pub struct VaultAccountParams {
    /// Vault id
    pub id: u8,

    /// PDA bump seeds
    pub bumps: Bumps,

    /// Whirlpool pubkey
    pub whirlpool_id: Pubkey,

    /// Pool input token_a mint address
    pub input_token_a_mint_pubkey: Pubkey,
    /// Pool input token_b mint address
    pub input_token_b_mint_pubkey: Pubkey,

    /// Fee percetange using FEE_SCALE
    pub fee: u64,
}

/// PDA bump seeds
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct Bumps {
    pub vault: u8,
    pub lp_token_mint: u8,
}

impl Bumps {
    pub const SIZE: usize = 1 + 1;
}

/// Position information
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct PositionInfo {
    /// Position pubkey
    pub pubkey: Pubkey,
    /// Position lower tick
    pub lower_tick: i32,
    /// Position upper tick
    pub upper_tick: i32,
}

impl PositionInfo {
    pub const SIZE: usize = 32 + 4 + 4;
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default, Debug)]
pub struct MarketRewardsInfo {
    /// Id of market associated
    pub id: MarketRewards,
    /// Pubkey of the rewards token mint
    pub rewards_mint: Pubkey,
    /// Destination account
    pub destination_token_account: Pubkey,
    /// Minimum number of lamports to receive during swap
    pub min_amount_out: u64,
}

impl MarketRewardsInfo {
    pub const SIZE: usize = MarketRewards::SIZE + 32 + 32 + 8;

    pub fn validate(
        &self,
        destination_mint: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
    ) -> Result<()> {
        if self.rewards_mint == Pubkey::default() {
            return Ok(());
        }

        match self.id {
            MarketRewards::NotSet => {}
            MarketRewards::Transfer => {
                require!(
                    self.rewards_mint == destination_mint,
                    ErrorCode::MarketInvalidDestination
                )
            }
            _ => {
                require!(
                    self.rewards_mint != token_a_mint && self.rewards_mint != token_b_mint,
                    ErrorCode::MarketInvalidSwapMint,
                );

                require!(
                    destination_mint == token_a_mint || destination_mint == token_b_mint,
                    ErrorCode::MarketInvalidDestination
                );

                require!(self.min_amount_out > 0, ErrorCode::MarketInvalidZeroAmount,);
            }
        };

        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum MarketRewards {
    NotSet,
    Transfer,
    OrcaV2,
    Whirlpool,
}

impl MarketRewards {
    pub const SIZE: usize = 1 + 1;
}

impl Default for MarketRewards {
    fn default() -> Self {
        MarketRewards::NotSet
    }
}
