/// Generate signed seeds for the vault account
macro_rules! generate_seeds {
    ($account:expr) => {
        &[
            "vault".as_ref(),
            $account.input_token_a_mint_pubkey.as_ref(),
            $account.input_token_b_mint_pubkey.as_ref(),
            &[$account.bumps.vault],
        ]
    };
}
pub(crate) use generate_seeds;
