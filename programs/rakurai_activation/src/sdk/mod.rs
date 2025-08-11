pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::{RakuraiActivationAccount, RakuraiActivationConfigAccount};

/// Derives the PDA (Program Derived Address) for a specific Rakurai activation account,
/// based on the given identity public key and the program ID.
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

/// Derives the PDA for the global Rakurai config account.
pub fn derive_config_account_address(rakurai_activation_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[RakuraiActivationConfigAccount::SEED],
        rakurai_activation_program_id,
    )
}
