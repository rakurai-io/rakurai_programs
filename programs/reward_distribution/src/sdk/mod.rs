pub mod instruction;

use anchor_lang::{prelude::Pubkey, solana_program::clock::Epoch};

use crate::{
    PartnerShareAccount, PartnerShareKind, RewardCollectionAccount,
    RewardDistributionConfigAccount,
};

/// Derives the PDA for a reward collection account using vote pubkey and epoch.
/// Returns the PDA and the bump.
pub fn derive_reward_collection_account_address(
    reward_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RewardCollectionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        reward_distribution_program_id,
    )
}

/// Derives the PDA for the reward distribution config account.
/// Returns the PDA and the bump.
pub fn derive_config_account_address(reward_distribution_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[RewardDistributionConfigAccount::SEED],
        reward_distribution_program_id,
    )
}

/// Derives a partner share PDA: `[PARTNER_SHARE, kind, name, vote]`.
pub fn derive_partner_share_account_address(
    reward_distribution_program_id: &Pubkey,
    share_kind: PartnerShareKind,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &PartnerShareAccount::pda_seeds(share_kind, name, validator_vote),
        reward_distribution_program_id,
    )
}

/// Derives the PDA for a partner tip-share account.
pub fn derive_partner_tip_share_account_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_partner_share_account_address(
        reward_distribution_program_id,
        PartnerShareKind::Tip,
        name,
        validator_vote,
    )
}

/// Derives the PDA for a partner backrun-share account.
pub fn derive_partner_backrun_share_account_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_partner_share_account_address(
        reward_distribution_program_id,
        PartnerShareKind::Backrun,
        name,
        validator_vote,
    )
}
