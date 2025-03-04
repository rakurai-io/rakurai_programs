use {crate::ErrorCode::AccountValidationFailure, anchor_lang::prelude::*, std::mem::size_of};

#[account]
#[derive(Default)]
pub struct Config {
    pub authority: Pubkey,
    pub block_builder_authority: Pubkey,
    pub max_validator_commission_bps: u16,
    pub block_builder_commission_bps: u16,
    pub block_builder_commission_account: Pubkey,
    pub bump: u8,
}

const HEADER_SIZE: usize = 8;

impl Config {
    pub const SEED: &'static [u8] = b"CONFIG_ACCOUNT";
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    pub fn validate(&self) -> Result<()> {
        const MAX_COMMISSION_BPS: u16 = 10000;
        let default_pubkey = Pubkey::default();

        if self.block_builder_commission_account == default_pubkey
            || self.block_builder_authority == default_pubkey
        {
            return Err(AccountValidationFailure.into());
        }

        if self.max_validator_commission_bps > MAX_COMMISSION_BPS
            || self.block_builder_commission_bps > MAX_COMMISSION_BPS
            || (self.block_builder_commission_bps + self.max_validator_commission_bps)
                > MAX_COMMISSION_BPS
        {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }
}