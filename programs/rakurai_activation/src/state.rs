use {
    crate::ErrorCode::{AccountValidationFailure, ArithmeticError, MaxCommissionBpsExceeded},
    anchor_lang::prelude::*,
    std::mem::size_of,
};

#[account]
#[derive(Default, InitSpace)]
pub struct RakuraiActivationConfigAccount {
    pub authority: Pubkey,
    pub block_builder_authority: Pubkey,
    pub block_builder_commission_bps: u16,
    pub block_builder_commission_account: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(Default, InitSpace)]
pub struct RakuraiActivationAccount {
    pub is_enabled: bool,
    pub proposer: Option<Pubkey>,
    pub validator_authority: Pubkey,
    pub validator_commission_bps: u16,
    pub validator_identity_pubkey: Pubkey,
    pub block_builder_authority: Pubkey,
    pub block_builder_commission_bps: u16,
    pub block_builder_commission_account: Pubkey,
    pub bump: u8,
    pub hash: Option<[u8; 64]>,
}

const HEADER_SIZE: usize = 8;

impl RakuraiActivationConfigAccount {
    pub const SEED: &'static [u8] = b"ACTIVATION_CONFIG_ACCOUNT";
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    pub fn validate(&self) -> Result<()> {
        const MAX_COMMISSION_BPS: u16 = 10000;
        let default_pubkey = Pubkey::default();

        if self.block_builder_commission_account == default_pubkey
            || self.block_builder_authority == default_pubkey
        {
            return Err(AccountValidationFailure.into());
        }

        if self.block_builder_commission_bps > MAX_COMMISSION_BPS {
            return Err(MaxCommissionBpsExceeded.into());
        }

        Ok(())
    }
}

impl RakuraiActivationAccount {
    pub const SEED: &'static [u8] = b"RAKURAI_ACTIVATION_ACCOUNT";

    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    pub fn validate(&self) -> Result<()> {
        let default_pubkey = Pubkey::default();
        if self.validator_identity_pubkey == default_pubkey
            || self.validator_authority == default_pubkey
            || self.block_builder_commission_account == default_pubkey
        {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    pub fn claim_expired(from: AccountInfo, to: AccountInfo) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent_lamports = rent.minimum_balance(from.data_len());

        let amount = from
            .lamports()
            .checked_sub(min_rent_lamports)
            .ok_or(ArithmeticError)?;
        Self::transfer_lamports(from, to, amount)?;

        Ok(amount)
    }

    fn transfer_lamports(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        **from.try_borrow_mut_lamports()? =
            from.lamports().checked_sub(amount).ok_or(ArithmeticError)?;
        **to.try_borrow_mut_lamports()? =
            to.lamports().checked_add(amount).ok_or(ArithmeticError)?;

        Ok(())
    }
}
