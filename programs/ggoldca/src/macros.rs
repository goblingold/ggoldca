/// Generate signed seeds for the vault account
macro_rules! generate_seeds {
    ($account:expr) => {
        &[
            "vault".as_ref(),
            &[$account.vault_id][..],
            $account.whirlpool_id.as_ref(),
            &[$account.bumps.vault],
        ]
    };
}
pub(crate) use generate_seeds;
