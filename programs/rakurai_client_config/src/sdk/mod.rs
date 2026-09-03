pub mod instruction;

use anchor_lang::prelude::Pubkey;

use crate::state::{
    CONFIG_STAGING_SEED, GLOBAL_CONFIG_SEED, STAGING_TAG_GLOBAL, STAGING_TAG_PROPOSAL,
    STAGING_TAG_VALIDATOR, VALIDATOR_CONFIG_SEED, VALIDATOR_PROPOSAL_SEED,
};

pub use crate::state::{
    effective_config, BlockEngineConfig, BlockEngineEntryV1, BlockEngineV1, Config, ConfigLimits,
    ConfigLimitsV1, ConfigStaging, ConfigV1, ConfigV2, GlobalConfig, P2cConfig, P2cEntryV1, P2cV1,
    Uuid, ValidatorConfig, ValidatorProposal, VirtualPriorityConfig, VirtualPriorityEntryV1,
    VirtualPriorityV1, ABSOLUTE_MAX_SETS_PER_SECTION, ABSOLUTE_MAX_URLS_PER_SET,
    ABSOLUTE_MAX_URL_LEN, ABSOLUTE_MAX_VP_ENTRIES_PER_SET, CONFIG_STAGING_SEED as STAGING_SEED,
    GLOBAL_CONFIG_SEED as GLOBAL_SEED, MAX_STAGING_BYTES, NAME_LEN, STAGING_KIND_GLOBAL,
    STAGING_KIND_PROPOSAL, STAGING_KIND_VALIDATOR, VALIDATOR_CONFIG_SEED as VALIDATOR_SEED,
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

pub fn derive_global_staging_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_STAGING_SEED, STAGING_TAG_GLOBAL], program_id)
}

pub fn derive_validator_staging_address(program_id: &Pubkey, vote: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CONFIG_STAGING_SEED, STAGING_TAG_VALIDATOR, vote.as_ref()],
        program_id,
    )
}

pub fn derive_proposal_staging_address(program_id: &Pubkey, vote: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CONFIG_STAGING_SEED, STAGING_TAG_PROPOSAL, vote.as_ref()],
        program_id,
    )
}

pub fn name_from_str(s: &str) -> Uuid {
    Uuid::from_str_truncated(s)
}
