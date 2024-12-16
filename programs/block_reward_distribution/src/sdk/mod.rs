pub mod instruction;

use anchor_lang::{prelude::Pubkey, solana_program::clock::Epoch};

use crate::{BlockRewardDistributionAccount, Config};

pub fn derive_block_reward_distribution_account_address(
    block_reward_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            BlockRewardDistributionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        block_reward_distribution_program_id,
    )
}

pub fn derive_config_account_address(
    block_reward_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], block_reward_distribution_program_id)
}
