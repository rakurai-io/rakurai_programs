pub mod instruction;

use anchor_lang::{prelude::Pubkey, solana_program::clock::Epoch};

use crate::{
    RevenueKind, RevenueShareAccount, RevenueShareAccountV1, RewardCollectionAccount,
    RewardDistributionConfigAccount, TipsAndMevShareConfigAccount,
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

/// Derives the PDA for the tips-and-mev-share config singleton.
pub fn derive_tips_and_mev_share_config_address(
    reward_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TipsAndMevShareConfigAccount::SEED],
        reward_distribution_program_id,
    )
}

/// Derives a legacy revenue share PDA: `[REVENUE_SHARE, TIP|MEV_SHARE, name, vote]`.
pub fn derive_revenue_share_account_address(
    reward_distribution_program_id: &Pubkey,
    share_kind: RevenueKind,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &RevenueShareAccount::pda_seeds(share_kind, name, validator_vote),
        reward_distribution_program_id,
    )
}

/// Derives a TCAV1 / MCAV1 PDA: `[REVENUE_SHARE_V1, TIP|MEV_SHARE, name, vote]`.
pub fn derive_revenue_share_account_v1_address(
    reward_distribution_program_id: &Pubkey,
    share_kind: RevenueKind,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &RevenueShareAccountV1::pda_seeds_v1(share_kind, name, validator_vote),
        reward_distribution_program_id,
    )
}

/// Derives the PDA for a legacy tip revenue share account (TCA).
/// Seeds: `[REVENUE_SHARE, TIP, name, vote]`.
pub fn derive_tip_collection_account_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_revenue_share_account_address(
        reward_distribution_program_id,
        RevenueKind::Tip,
        name,
        validator_vote,
    )
}

/// Derives the PDA for a tip TCAV1.
/// Seeds: `[REVENUE_SHARE_V1, TIP, name, vote]`.
pub fn derive_tip_collection_account_v1_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_revenue_share_account_v1_address(
        reward_distribution_program_id,
        RevenueKind::Tip,
        name,
        validator_vote,
    )
}

/// Derives the PDA for a legacy mev-share revenue share account (MCA).
/// Seeds: `[REVENUE_SHARE, MEV_SHARE, name, vote]`.
pub fn derive_mev_share_collection_account_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_revenue_share_account_address(
        reward_distribution_program_id,
        RevenueKind::MevShare,
        name,
        validator_vote,
    )
}

/// Derives the PDA for an MCAV1.
/// Seeds: `[REVENUE_SHARE_V1, MEV_SHARE, name, vote]`.
pub fn derive_mev_share_collection_account_v1_address(
    reward_distribution_program_id: &Pubkey,
    name: &[u8; 32],
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_revenue_share_account_v1_address(
        reward_distribution_program_id,
        RevenueKind::MevShare,
        name,
        validator_vote,
    )
}
