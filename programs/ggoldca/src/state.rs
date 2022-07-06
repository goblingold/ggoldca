use anchor_lang::prelude::*;

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
}

impl VaultAccount {
    pub const SIZE: usize = Bumps::SIZE + 32 + 32 + 32 + 32;

    /// Initialize a new vault
    pub fn init(params: InitVaultAccountParams) -> Self {
        Self {
            bumps: params.bumps,
            whirlpool_id: params.whirlpool_id,
            input_token_a_mint_pubkey: params.input_token_a_mint_pubkey,
            input_token_b_mint_pubkey: params.input_token_b_mint_pubkey,
            dao_treasury_lp_token_account: params.dao_treasury_lp_token_account,
        }
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
