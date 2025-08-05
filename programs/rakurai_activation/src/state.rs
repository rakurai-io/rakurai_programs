use {
    crate::ErrorCode::{AccountValidationFailure, ArithmeticError, MaxCommissionBpsExceeded},
    anchor_lang::prelude::*,
    std::mem::size_of,
};

#[account]
#[derive(Default)]
pub struct RakuraiActivationConfigAccount {
    /// Primary authority over this PDA.
    pub authority: Pubkey,

    /// Authority related to block building logic.
    pub block_builder_authority: Pubkey,

    /// Commission charged by block builder (in basis points).
    pub block_builder_commission_bps: u16,

    /// Account where the commission is sent.
    pub block_builder_commission_account: Pubkey,

    /// Bump seed for PDA.
    pub bump: u8,
}

#[account]
#[derive(Default)]
pub struct RakuraiActivationAccount {
    /// Whether the activation is enabled.
    pub is_enabled: bool,

    /// Who proposed this change (optional).
    pub proposer: Option<Pubkey>,

    /// Main validator's signing authority.
    pub validator_authority: Pubkey,

    /// Validator commission in basis points.
    pub validator_commission_bps: u16,

    /// Block builder commission in basis points.
    pub block_builder_commission_bps: u16,

    /// Bump seed for PDA.
    pub bump: u8,

    /// Optional hash.
    pub hash: Option<[u8; 64]>,
}

const HEADER_SIZE: usize = 8;
const MAX_COMMISSION_BPS: u16 = 10_000;

impl RakuraiActivationConfigAccount {
    /// Seed used to derive PDA address for RakuraiActivationConfigAccount.
    pub const SEED: &'static [u8] = b"ACTIVATION_CONFIG_ACCOUNT";

    /// Total space required for the account: 8 bytes header + serialized struct size.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    /// Validates fields of the account.
    pub fn validate(&self) -> Result<()> {
        let default_pubkey = Pubkey::default();

        if self.block_builder_commission_account == default_pubkey {
            return Err(AccountValidationFailure.into());
        }

        if self.block_builder_commission_bps > MAX_COMMISSION_BPS {
            return Err(MaxCommissionBpsExceeded.into());
        }

        Ok(())
    }
}

impl RakuraiActivationAccount {
    /// Seed used for PDA derivation.
    pub const SEED: &'static [u8] = b"RAKURAI_ACTIVATION_ACCOUNT";

    /// Total size including header.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    /// Validates account fields.
    pub fn validate(&self) -> Result<()> {
        let default_pubkey = Pubkey::default();
        if self.validator_authority == default_pubkey {
            return Err(AccountValidationFailure.into());
        }
        if self.block_builder_commission_bps > MAX_COMMISSION_BPS
            && self.validator_commission_bps > MAX_COMMISSION_BPS
            && (self.validator_commission_bps + self.block_builder_commission_bps)
                > MAX_COMMISSION_BPS
        {
            return Err(MaxCommissionBpsExceeded.into());
        }

        Ok(())
    }

    /// Drains lamports (excluding rent) from `from` to `to`.
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

    /// Transfers lamports between accounts.
    fn transfer_lamports(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        **from.try_borrow_mut_lamports()? =
            from.lamports().checked_sub(amount).ok_or(ArithmeticError)?;
        **to.try_borrow_mut_lamports()? =
            to.lamports().checked_add(amount).ok_or(ArithmeticError)?;

        Ok(())
    }
}
