pub mod instruction;

use anchor_lang::{prelude::Pubkey, solana_program::clock::Epoch};

use crate::{Config, RewardDistributionAccount};

pub fn derive_reward_distribution_account_address(
    reward_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RewardDistributionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        reward_distribution_program_id,
    )
}

pub fn derive_config_account_address(reward_distribution_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], reward_distribution_program_id)
}
