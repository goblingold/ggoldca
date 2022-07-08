use crate::error::ErrorCode;
use crate::interface::*;
use crate::macros::generate_seeds;
use crate::state::VaultAccount;
use crate::VAULT_ACCOUNT_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang_for_whirlpool::{
    context::CpiContext as CpiContextForWhirlpool, AccountDeserialize,
};
use anchor_spl::token::{Token, TokenAccount};
use std::borrow::Borrow;
use whirlpool::cpi::accounts::{CollectFees, CollectReward, UpdateFeesAndRewards};

#[derive(Accounts)]
pub struct CollectFeesAndRewards<'info> {
    pub user_signer: Signer<'info>,
    #[account(
        seeds = [VAULT_ACCOUNT_SEED, vault_account.whirlpool_id.as_ref()],
        bump = vault_account.bumps.vault
    )]
    pub vault_account: Box<Account<'info, VaultAccount>>,

    #[account(address = whirlpool::ID)]
    /// CHECK: address is checked
    pub whirlpool_program_id: AccountInfo<'info>,

    #[account(mut)]
    pub token_owner_account_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_owner_account_b: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_a: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: whirlpool cpi
    pub token_vault_b: AccountInfo<'info>,

    #[account(
        constraint = position.whirlpool.key() == vault_account.whirlpool_id.key(),
        constraint = position.position.key() == vault_account.active_position_key() @ ErrorCode::PositionNotActive,
    )]
    pub position: PositionAccounts<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> CollectFeesAndRewards<'info> {
    fn update_fees_and_rewards_ctx(
        &self,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, UpdateFeesAndRewards<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            UpdateFeesAndRewards {
                whirlpool: self.position.whirlpool.to_account_info(),
                position: self.position.position.to_account_info(),
                tick_array_lower: self.position.tick_array_lower.to_account_info(),
                tick_array_upper: self.position.tick_array_upper.to_account_info(),
            },
        )
    }

    fn collect_fees_ctx(&self) -> CpiContextForWhirlpool<'_, '_, '_, 'info, CollectFees<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            CollectFees {
                whirlpool: self.position.whirlpool.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                token_owner_account_a: self.token_owner_account_a.to_account_info(),
                token_owner_account_b: self.token_owner_account_b.to_account_info(),
                token_vault_a: self.token_vault_a.to_account_info(),
                token_vault_b: self.token_vault_b.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    fn collect_rewards_ctx(
        &self,
        reward_owner_account: AccountInfo<'info>,
        reward_vault: AccountInfo<'info>,
    ) -> CpiContextForWhirlpool<'_, '_, '_, 'info, CollectReward<'info>> {
        CpiContextForWhirlpool::new(
            self.whirlpool_program_id.to_account_info(),
            CollectReward {
                whirlpool: self.position.whirlpool.to_account_info(),
                position_authority: self.vault_account.to_account_info(),
                position: self.position.position.to_account_info(),
                position_token_account: self.position.position_token_account.to_account_info(),
                reward_vault,
                reward_owner_account,
                token_program: self.token_program.to_account_info(),
            },
        )
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, CollectFeesAndRewards<'info>>) -> Result<()> {
    let seeds = generate_seeds!(ctx.accounts.vault_account);
    let signer = &[&seeds[..]];

    whirlpool::cpi::update_fees_and_rewards(ctx.accounts.update_fees_and_rewards_ctx())?;
    whirlpool::cpi::collect_fees(ctx.accounts.collect_fees_ctx().with_signer(signer))?;

    let rewards_mints: Vec<Pubkey> = {
        let acc_data_slice: &[u8] = &ctx.accounts.position.whirlpool.try_borrow_data()?;
        let pool =
            whirlpool::state::whirlpool::Whirlpool::try_deserialize(&mut acc_data_slice.borrow())?;

        pool.reward_infos
            .iter()
            .map(|info| info.mint)
            .filter(|key| key != &Pubkey::default())
            .collect::<_>()
    };

    let num_rewards = rewards_mints.len();

    require!(
        ctx.remaining_accounts.len() == 2 * num_rewards,
        ErrorCode::InvalidRemainingAccounts
    );

    let (vault_rewards_accs, whirlpool_rewards_accs) = ctx.remaining_accounts.split_at(num_rewards);

    for i in 0..num_rewards {
        let vault_token_acc = Account::<TokenAccount>::try_from(&vault_rewards_accs[i])?;

        require!(
            vault_token_acc.mint == rewards_mints[i],
            anchor_lang::prelude::ErrorCode::ConstraintTokenMint
        );

        require!(
            vault_token_acc.owner == ctx.accounts.vault_account.key(),
            anchor_lang::prelude::ErrorCode::ConstraintTokenOwner
        );

        whirlpool::cpi::collect_reward(
            ctx.accounts
                .collect_rewards_ctx(
                    vault_token_acc.to_account_info(),
                    whirlpool_rewards_accs[i].to_account_info(),
                )
                .with_signer(signer),
            i as u8,
        )?;
    }

    Ok(())
}
