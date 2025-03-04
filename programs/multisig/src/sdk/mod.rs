pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::Config;

pub fn derive_config_account_address(multisig_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], multisig_program_id)
}
