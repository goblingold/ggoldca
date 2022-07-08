use anchor_lang::prelude::*;

/// Number of simultaneous positions allowed
pub const MAX_POSITIONS: usize = 2;

/// Strategy vault account
#[account]
#[derive(Default, Debug)]
pub struct VaultAccount {
    /// PDA bump seeds
    pub bumps: Bumps,

    /// Whirlpool pubkey
    pub whirlpool_id: Pubkey,
    /// Pool input token_a mint address
    pub input_token_a_mint_pubkey: Pubkey,
    /// Pool input token_b mint address
    pub input_token_b_mint_pubkey: Pubkey,

    /// Destination fee account
    pub dao_treasury_lp_token_account: Pubkey,

    /// Information about the opened positions (max = MAX_POSITIONS)
    pub positions: Vec<PositionInfo>,
}

impl VaultAccount {
    pub const SIZE: usize =
        Bumps::SIZE + 32 + 32 + 32 + 32 + 4 + MAX_POSITIONS * PositionInfo::SIZE;

    /// Initialize a new vault
    pub fn init(params: InitVaultAccountParams) -> Self {
        Self {
            bumps: params.bumps,
            whirlpool_id: params.whirlpool_id,
            input_token_a_mint_pubkey: params.input_token_a_mint_pubkey,
            input_token_b_mint_pubkey: params.input_token_b_mint_pubkey,
            dao_treasury_lp_token_account: params.dao_treasury_lp_token_account,
            ..Self::default()
        }
    }

    /// Check the existence of a position
    pub fn position_exists(&self, tick_lower_index: i32, tick_upper_index: i32) -> bool {
        return self
            .positions
            .iter()
            .any(|pos| pos.lower_tick == tick_lower_index && pos.upper_tick == tick_upper_index);
    }

    /// Check if the given pubkey is a valid position
    pub fn position_address_exists(&self, key: Pubkey) -> bool {
        return self.positions.iter().any(|pos| pos.pubkey == key);
    }

    /// Return the current active position pubkey
    pub fn active_position_key(&self) -> Pubkey {
        return self.positions[0].pubkey;
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
    /// PDA bump seeds
    pub bumps: Bumps,

    /// Whirlpool pubkey
    pub whirlpool_id: Pubkey,

    /// Pool input token_a mint address
    pub input_token_a_mint_pubkey: Pubkey,
    /// Pool input token_b mint address
    pub input_token_b_mint_pubkey: Pubkey,
    /// Destination fee account
    pub dao_treasury_lp_token_account: Pubkey,
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
