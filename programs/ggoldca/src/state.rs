use anchor_lang::prelude::*;

/// Strategy vault account
#[account]
#[derive(Default)]
pub struct VaultAccount {
    /// PDA bump seeds
    pub bumps: Bumps,

    /// Pool input token_a mint address
    pub input_token_a_mint_pubkey: Pubkey,
    /// Pool input token_b mint address
    pub input_token_b_mint_pubkey: Pubkey,
    /// Destination fee account
    pub dao_treasury_lp_token_account: Pubkey,
}

impl VaultAccount {
    pub const SIZE: usize = Bumps::SIZE + 1_000;
}

/// PDA bump seeds
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct Bumps {
    pub vault: u8,
    pub lp_token_mint: u8,
}

impl Bumps {
    pub const SIZE: usize = 1 + 1;
}
