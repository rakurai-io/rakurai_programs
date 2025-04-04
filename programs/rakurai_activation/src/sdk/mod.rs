pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::{RakuraiActivationAccount, RakuraiActivationConfigAccount};

pub fn derive_multisig_account_address(
    multisig_program_id: &Pubkey,
    identity_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RakuraiActivationAccount::SEED,
            identity_pubkey.to_bytes().as_ref(),
        ],
        multisig_program_id,
    )
}

pub fn derive_config_account_address(multisig_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RakuraiActivationConfigAccount::SEED], multisig_program_id)
}
