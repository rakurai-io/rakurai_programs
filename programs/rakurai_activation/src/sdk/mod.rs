pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::{RakuraiActivationAccount, RakuraiActivationConfigAccount};

pub fn derive_activation_account_address(
    rakurai_activation_program_id: &Pubkey,
    identity_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RakuraiActivationAccount::SEED,
            identity_pubkey.to_bytes().as_ref(),
        ],
        rakurai_activation_program_id,
    )
}

pub fn derive_config_account_address(rakurai_activation_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[RakuraiActivationConfigAccount::SEED],
        rakurai_activation_program_id,
    )
}
