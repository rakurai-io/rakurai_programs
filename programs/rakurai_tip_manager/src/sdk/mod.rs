use anchor_lang::prelude::Pubkey;

use crate::{
    RAKURAI_REVENUE_NAME, RAKURAI_TIP_ACCOUNT_0_SEED, RAKURAI_TIP_ACCOUNT_1_SEED,
    RAKURAI_TIP_ACCOUNT_2_SEED, RAKURAI_TIP_ACCOUNT_3_SEED, RAKURAI_TIP_ACCOUNT_4_SEED,
    RAKURAI_TIP_ACCOUNT_5_SEED, RAKURAI_TIP_ACCOUNT_6_SEED, RAKURAI_TIP_ACCOUNT_7_SEED,
    RECORD_AUTHORITY_SEED, TIP_MANAGER_CONFIG_ACCOUNT_SEED,
};
use reward_distribution::sdk::{
    derive_tip_collection_account_address, derive_tip_collection_account_v1_address,
};

pub mod instruction;

pub fn derive_rakurai_tip_payment_account_pdas(program_id: &Pubkey) -> Vec<(Pubkey, u8)> {
    vec![
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_0_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_1_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_2_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_3_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_4_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_5_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_6_SEED], program_id),
        Pubkey::find_program_address(&[RAKURAI_TIP_ACCOUNT_7_SEED], program_id),
    ]
}

pub fn derive_rakurai_tip_manager_config_account_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TIP_MANAGER_CONFIG_ACCOUNT_SEED], program_id)
}

/// Derives the tip manager PDA used to sign `reward_distribution::record_revenue` / `_v1` CPIs.
/// Set this address as the TCA / TCAV1 `record_authority` at init.
pub fn derive_record_authority_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RECORD_AUTHORITY_SEED], program_id)
}

/// Derives the legacy Rakurai tip TCA PDA.
/// Seeds: `[REVENUE_SHARE, TIP, RAKURAI_REVENUE_NAME, vote]`.
pub fn derive_rakurai_tip_collection_address(
    reward_distribution_program_id: &Pubkey,
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_tip_collection_account_address(
        reward_distribution_program_id,
        &RAKURAI_REVENUE_NAME,
        validator_vote,
    )
}

/// Derives the Rakurai tip TCAV1 PDA.
/// Seeds: `[REVENUE_SHARE_V1, TIP, RAKURAI_REVENUE_NAME, vote]`.
pub fn derive_rakurai_tip_collection_v1_address(
    reward_distribution_program_id: &Pubkey,
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_tip_collection_account_v1_address(
        reward_distribution_program_id,
        &RAKURAI_REVENUE_NAME,
        validator_vote,
    )
}
