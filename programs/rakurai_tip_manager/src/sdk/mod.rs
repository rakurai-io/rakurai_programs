use anchor_lang::prelude::Pubkey;

use crate::{
    RAKURAI_PARTNER_TIP_SHARE_NAME, RAKURAI_TIP_ACCOUNT_0_SEED, RAKURAI_TIP_ACCOUNT_1_SEED,
    RAKURAI_TIP_ACCOUNT_2_SEED, RAKURAI_TIP_ACCOUNT_3_SEED, RAKURAI_TIP_ACCOUNT_4_SEED,
    RAKURAI_TIP_ACCOUNT_5_SEED, RAKURAI_TIP_ACCOUNT_6_SEED, RAKURAI_TIP_ACCOUNT_7_SEED,
    TIP_MANAGER_CONFIG_ACCOUNT_SEED,
};
use reward_distribution::sdk::derive_partner_tip_share_account_address;

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

/// Derives the Rakurai partner tip-share PDA for a validator vote (reward_distribution program).
pub fn derive_rakurai_partner_tip_share_address(
    reward_distribution_program_id: &Pubkey,
    validator_vote: &Pubkey,
) -> (Pubkey, u8) {
    derive_partner_tip_share_account_address(
        reward_distribution_program_id,
        &RAKURAI_PARTNER_TIP_SHARE_NAME,
        validator_vote,
    )
}
