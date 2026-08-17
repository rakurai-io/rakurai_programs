pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::state::{GLOBAL_CONFIG_SEED, VALIDATOR_CONFIG_SEED, VALIDATOR_PROPOSAL_SEED};

pub use crate::state::{
    union_configs, BlockEngineConfig, BlockEngineEntryV1, BlockEngineV1, Config, ConfigV1,
    GlobalConfig, P2cConfig, P2cEntryV1, P2cV1, Uuid, ValidatorConfig, ValidatorProposal,
    VirtualPriorityConfig, VirtualPriorityEntryV1, VirtualPriorityV1,
    GLOBAL_CONFIG_SEED as GLOBAL_SEED, NAME_LEN, VALIDATOR_CONFIG_SEED as VALIDATOR_SEED,
    VALIDATOR_PROPOSAL_SEED as PROPOSAL_SEED,
};

pub fn derive_global_config_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], program_id)
}

pub fn derive_validator_config_address(program_id: &Pubkey, vote: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VALIDATOR_CONFIG_SEED, vote.as_ref()], program_id)
}

pub fn derive_validator_proposal_address(program_id: &Pubkey, vote: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VALIDATOR_PROPOSAL_SEED, vote.as_ref()], program_id)
}

pub fn name_from_str(s: &str) -> Uuid {
    Uuid::from_str_truncated(s)
}
