use anchor_lang::prelude::*;

/// Number of simultaneous positions allowed
pub const MAX_POSITIONS: usize = 3;

/// Number of whirlpool rewards (from whirlpool::state::whirlpool::NUM_REWARDS)
pub const WHIRLPOOL_NUM_REWARDS: usize = 3;

/// Strategy vault account
#[account]
#[derive(Default, Debug)]
pub struct VaultAccount {
    /// Vault number for a given whirlpool
    pub vault_id: u8,

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

    /// Additional padding
    pub _padding: [u64; 10],

    /// The market where to sell the rewards
    pub market_rewards: [MarketRewardsInfo; WHIRLPOOL_NUM_REWARDS],
    /// Information about the opened positions (max = MAX_POSITIONS)
    pub positions: Vec<PositionInfo>,
}

impl VaultAccount {
    pub const SIZE: usize = 1
        + Bumps::SIZE
        + 32
        + 32
        + 32
        + 16
        + 8
        + 8
        + 8
        + 8 * 10
        + 4
        + WHIRLPOOL_NUM_REWARDS * MarketRewardsInfo::SIZE
        + MAX_POSITIONS * PositionInfo::SIZE;

    /// Initialize a new vault
    pub fn init(params: InitVaultAccountParams) -> Self {
        Self {
            bumps: params.bumps,
            whirlpool_id: params.whirlpool_id,
            input_token_a_mint_pubkey: params.input_token_a_mint_pubkey,
            input_token_b_mint_pubkey: params.input_token_b_mint_pubkey,
            fee: params.fee,
            market_rewards: params.market_rewards_info,
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

/// Initialize a new vault
pub struct InitVaultAccountParams {
    /// Vault id
    pub vault_id: u8,

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
    /// Market rewards infos
    pub market_rewards_info: [MarketRewardsInfo; WHIRLPOOL_NUM_REWARDS],
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
    /// Pubkey of the rewards token mint
    pub rewards_mint: Pubkey,
    /// Pubkey of the mint output to swap the rewards for
    pub is_destination_token_a: bool,
    /// Id of market associated
    pub id: MarketRewards,
}

impl MarketRewardsInfo {
    pub const SIZE: usize = 32 + 1 + 2;
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum MarketRewards {
    NotSet,
    OrcaV2,
    Whirlpool,
}

impl Default for MarketRewards {
    fn default() -> Self {
        MarketRewards::NotSet
    }
}
