pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::{Config, MultiSigAccount};

pub fn derive_multisig_account_address(
    multisig_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MultiSigAccount::SEED, vote_pubkey.to_bytes().as_ref()],
        multisig_program_id,
    )
}

pub fn derive_config_account_address(multisig_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], multisig_program_id)
}
