use crate::ErrorCode::{
    AccountValidationFailure, ArithmeticError, EpochAlreadyClaimed, EpochEntryNotFound,
    InvalidPartnerName, InvalidPartnerShareEpochCapacity, MaxCommissionFeeBpsExceeded,
    TipBackrunManagerNotConfigured,
};
use anchor_lang::prelude::*;
use std::mem::size_of;

/// Stores configuration for the reward distribution program.
#[account]
#[derive(Default)]
pub struct RewardDistributionConfigAccount {
    /// Authorized updater of the config.
    pub authority: Pubkey,
    /// Number of epochs the collection account is valid.
    pub num_epochs_valid: u64,
    /// Max allowed validator commission (basis points).
    pub max_commission_bps: u16,
    /// PDA bump.
    pub bump: u8,
    /// If enabled, Block Builder will also deduct its commission from the validator’s MEV commission.
    pub block_builder_commission_on_mev_commission_enabled: Option<bool>,
    /// Authority that may create partner tip/backrun share accounts and manage claims. `None` disables partner share creation.
    pub tip_backrun_manager_authority: Option<Pubkey>,
}

/// Stores validator reward collection account data for a given epoch.
#[account]
#[derive(Default)]
pub struct RewardCollectionAccount {
    /// Validator's vote account.
    pub validator_vote_account: Pubkey,
    /// Authorized uploader of the Merkle root.
    pub merkle_root_upload_authority: Pubkey,
    /// Optional Merkle root used for claims.
    pub merkle_root: Option<MerkleRoot>,
    /// Epoch when account was initialized.
    pub creation_epoch: u64,
    /// Commission on Block Rewards taken by validator specified in basis points.
    pub block_reward_commission_bps: u16,
    /// Commission on block rewards & MEV tips specified in basis points; deducted if enabled.
    pub block_builder_commission_bps: u16,
    /// Account receiving block builder commission.
    pub block_builder_commission_account: Pubkey,
    /// Epoch when claims expire.
    pub expires_at: u64,
    /// Who initialized the account (validator identity).
    pub initializer: Pubkey,
    /// PDA bump.
    pub bump: u8,
    /// Amount of MEV commission deducted by Block Builder (if enabled).
    pub block_builder_mev_commission_deducted: Option<u64>,
}

/// Metadata about the Merkle root used for claims.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct MerkleRoot {
    /// Merkle root hash.
    pub root: [u8; 32],
    /// Max total funds claimable.
    pub max_total_claim: u64,
    /// Max number of nodes that can claim.
    pub max_num_nodes: u64,
    /// Funds already claimed.
    pub total_funds_claimed: u64,
    /// Number of nodes that have claimed.
    pub num_nodes_claimed: u64,
}

pub const MAX_PARTNER_SHARE_EPOCH_ENTRIES_CAP: usize = 32;

/// Per-epoch attributed amount.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct EpochAmountEntry {
    pub epoch: u64,
    pub amount: u64,
    pub claimed: bool,
    /// When true, this epoch's share is gets converted to block rewards.
    pub converted_to_block_reward: bool,
}

/// Per-partner epoch share ledger; grows until `max_epoch_entries` then overrides oldest epoch.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct PartnerShareLedger {
    pub entries: Vec<EpochAmountEntry>,
}

/// Partner share vault kind; included in PDA seeds after [`PartnerShareAccount::SEED`].
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartnerShareKind {
    Tip,
    Backrun,
}

impl PartnerShareKind {
    pub const TIP_SEED: &'static [u8] = b"TIP";
    pub const BACKRUN_SEED: &'static [u8] = b"BACKRUN";

    pub fn seed(self) -> &'static [u8] {
        match self {
            Self::Tip => Self::TIP_SEED,
            Self::Backrun => Self::BACKRUN_SEED,
        }
    }
}

/// Partner tip/backrun share vault per validator (accounting + lamport vault).
#[account]
pub struct PartnerShareAccount {
    /// Tip vs backrun; part of the PDA seeds with `name` and `validator_vote`.
    pub share_kind: PartnerShareKind,
    /// UTF-8 padded partner label (used in PDA seeds).
    pub name: [u8; 32],
    pub validator_vote: Pubkey,
    /// Claims revenue and closes the account.
    pub manager_authority: Pubkey,
    /// Signs `record_partner_*_share`.
    pub record_authority: Pubkey,
    /// Max distinct epochs in `ledger`.
    pub max_epoch_entries: u8,
    /// Commission on partner share claims (basis points); remainder goes to validator identity.
    pub commission_bps: u16,
    /// Receives the commission portion on claim.
    pub commission_account: Pubkey,
    /// When true, recorded share is converted to block rewards on claim.
    pub convert_to_block_rewards: bool,
    pub ledger: PartnerShareLedger,
    pub bump: u8,
}

/// Tip-share PDA; same account layout as [`PartnerBackrunShareAccount`].
pub type PartnerTipShareAccount = PartnerShareAccount;
/// Backrun-share PDA; same account layout as [`PartnerTipShareAccount`].
pub type PartnerBackrunShareAccount = PartnerShareAccount;

const HEADER_SIZE: usize = 8;
const MAX_COMMISSION_BPS: u16 = 10000;

/// Validates partner-share commission fields against config caps.
pub fn validate_partner_commission(
    commission_bps: u16,
    commission_account: Pubkey,
    max_commission_bps: u16,
) -> Result<()> {
    if commission_bps > max_commission_bps {
        return Err(MaxCommissionFeeBpsExceeded.into());
    }
    if commission_bps > 0 && commission_account == Pubkey::default() {
        return Err(AccountValidationFailure.into());
    }
    Ok(())
}

impl RewardDistributionConfigAccount {
    /// PDA seed for the config account.
    pub const SEED: &'static [u8] = b"RD_CONFIG_ACCOUNT";
    /// Account size for rent-exemption.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    /// Gets whether MEV commission is enabled, defaulting to false for old configs.
    pub fn is_mev_commission_enabled(&self) -> bool {
        self.block_builder_commission_on_mev_commission_enabled
            .unwrap_or(false)
    }

    /// Sets MEV commission enabled status.
    pub fn set_mev_commission_enabled(&mut self, enabled: bool) {
        self.block_builder_commission_on_mev_commission_enabled = Some(enabled);
    }

    /// Checks if MEV commission setting is configured.
    pub fn has_mev_commission_setting(&self) -> bool {
        self.block_builder_commission_on_mev_commission_enabled
            .is_some()
    }

    /// Returns the configured tip/backrun partner manager, if partner share creation is enabled.
    pub fn require_tip_backrun_manager_authority(&self) -> Result<Pubkey> {
        self.tip_backrun_manager_authority
            .filter(|key| *key != Pubkey::default())
            .ok_or(TipBackrunManagerNotConfigured.into())
    }

    /// Validates config constraints.
    pub fn validate(&self) -> Result<()> {
        const MAX_NUM_EPOCHS_VALID: u64 = 10;

        if self.num_epochs_valid == 0 || self.num_epochs_valid > MAX_NUM_EPOCHS_VALID {
            return Err(AccountValidationFailure.into());
        }

        if self.max_commission_bps > MAX_COMMISSION_BPS {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }
}

impl RewardCollectionAccount {
    /// PDA seed for collection accounts.
    pub const SEED: &'static [u8] = b"REWARD_COLLECTION_ACCOUNT";

    /// Account size for rent-exemption.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    /// Lamports available for claims after rent (balance minus minimum rent).
    pub fn spendable_lamports(lamports: u64, min_rent: u64) -> Result<u64> {
        lamports.checked_sub(min_rent).ok_or(ArithmeticError.into())
    }

    /// Validates that required fields are not default.
    pub fn validate(&self) -> Result<()> {
        let default_pubkey = Pubkey::default();
        if self.validator_vote_account == default_pubkey
            || self.merkle_root_upload_authority == default_pubkey
            || self.block_builder_commission_account == default_pubkey
        {
            return Err(AccountValidationFailure.into());
        }

        if self.initializer == default_pubkey {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    /// Claims all lamports from the account (except rent) on expiry.
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

    /// Claims a specified amount from the account.
    pub fn claim(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        Self::transfer_lamports(from, to, amount)
    }

    /// Internal helper to safely transfer lamports.
    pub fn transfer_lamports(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        // debit lamports
        **from.try_borrow_mut_lamports()? =
            from.lamports().checked_sub(amount).ok_or(ArithmeticError)?;
        // credit lamports
        **to.try_borrow_mut_lamports()? =
            to.lamports().checked_add(amount).ok_or(ArithmeticError)?;

        Ok(())
    }
}

impl PartnerShareLedger {
    pub fn get_mut(&mut self, epoch: u64) -> Result<&mut EpochAmountEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.epoch == epoch)
            .ok_or(EpochEntryNotFound.into())
    }

    pub fn add(
        &mut self,
        epoch: u64,
        amount: u64,
        capacity: usize,
        converted_to_block_reward: bool,
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        for entry in &mut self.entries {
            if entry.epoch == epoch {
                entry.amount = entry.amount.checked_add(amount).ok_or(ArithmeticError)?;
                entry.converted_to_block_reward = converted_to_block_reward;
                return Ok(());
            }
        }

        let new_entry = EpochAmountEntry {
            epoch,
            amount,
            claimed: false,
            converted_to_block_reward,
        };

        if self.entries.len() < capacity {
            self.entries.push(new_entry);
            return Ok(());
        }

        let len = self.entries.len();
        let mut oldest_idx = 0usize;
        let mut oldest_epoch = self.entries[0].epoch;
        for i in 1..len {
            if self.entries[i].epoch < oldest_epoch {
                oldest_epoch = self.entries[i].epoch;
                oldest_idx = i;
            }
        }

        self.entries[oldest_idx] = new_entry;
        Ok(())
    }

    pub fn mark_claimed(&mut self, epoch: u64) -> Result<()> {
        let entry = self.get_mut(epoch)?;
        if entry.claimed {
            return Err(EpochAlreadyClaimed.into());
        }
        entry.claimed = true;
        Ok(())
    }
}

impl PartnerShareAccount {
    pub const SEED: &'static [u8] = b"PARTNER_SHARE";

    pub fn pda_seeds<'a>(
        share_kind: PartnerShareKind,
        name: &'a [u8; 32],
        validator_vote: &'a Pubkey,
    ) -> [&'a [u8]; 4] {
        [
            Self::SEED,
            share_kind.seed(),
            name.as_ref(),
            validator_vote.as_ref(),
        ]
    }

    pub fn space_for(max_epoch_entries: usize) -> usize {
        HEADER_SIZE
            + 1
            + 32
            + 32
            + 32
            + 32
            + 1
            + 2
            + 32
            + 1
            + 4
            + max_epoch_entries * size_of::<EpochAmountEntry>()
            + 1
    }

    pub fn populate_on_init(
        &mut self,
        share_kind: PartnerShareKind,
        name: [u8; 32],
        validator_vote: Pubkey,
        manager_authority: Pubkey,
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        bump: u8,
    ) -> Result<()> {
        self.share_kind = share_kind;
        self.name = name;
        self.validator_vote = validator_vote;
        self.manager_authority = manager_authority;
        self.record_authority = record_authority;
        self.max_epoch_entries = max_epoch_entries;
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.convert_to_block_rewards = false;
        self.ledger = PartnerShareLedger::default();
        self.bump = bump;
        self.validate()
    }

    pub fn record_share(&mut self, epoch: u64, amount: u64) -> Result<()> {
        let capacity = self.max_epoch_entries as usize;
        self.ledger.add(
            epoch,
            amount,
            capacity,
            self.convert_to_block_rewards,
        )
    }

    pub fn update_commission(
        &mut self,
        commission_bps: u16,
        commission_account: Pubkey,
        convert_to_block_rewards: bool,
        manager_authority: Pubkey,
    ) -> Result<()> {
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.convert_to_block_rewards = convert_to_block_rewards;
        self.manager_authority = manager_authority;
        self.validate()
    }

    pub fn auth_record_signer(&self, signer: Pubkey) -> Result<()> {
        if signer != self.record_authority {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Ok(())
    }

    pub fn auth_manager_signer(&self, signer: Pubkey) -> Result<()> {
        if signer != self.manager_authority {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Ok(())
    }

    pub fn auth_manager_or_record_signer(&self, signer: Pubkey) -> Result<()> {
        if signer != self.manager_authority && signer != self.record_authority {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Ok(())
    }

    pub fn set_convert_to_block_rewards(&mut self, convert_to_block_rewards: bool) -> Result<()> {
        self.convert_to_block_rewards = convert_to_block_rewards;
        self.validate()
    }

    /// Validates init instruction args before the account is populated.
    pub fn validate_init_params(
        name: [u8; 32],
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        max_commission_bps: u16,
    ) -> Result<()> {
        if name == [0u8; 32] {
            return Err(InvalidPartnerName.into());
        }

        if record_authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        if max_epoch_entries == 0
            || max_epoch_entries as usize > MAX_PARTNER_SHARE_EPOCH_ENTRIES_CAP
        {
            return Err(InvalidPartnerShareEpochCapacity.into());
        }

        validate_partner_commission(commission_bps, commission_account, max_commission_bps)?;

        Ok(())
    }

    pub fn auth_initialize_payer(
        config: &RewardDistributionConfigAccount,
        payer: Pubkey,
        name: [u8; 32],
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
    ) -> Result<Pubkey> {
        let manager = config.require_tip_backrun_manager_authority()?;
        if payer != manager {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Self::validate_init_params(
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            config.max_commission_bps,
        )?;
        Ok(manager)
    }

    /// Validates init instruction args including manager authority.
    pub fn auth(
        name: [u8; 32],
        manager_authority: Pubkey,
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        max_commission_bps: u16,
    ) -> Result<()> {
        Self::validate_init_params(
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            max_commission_bps,
        )?;

        if manager_authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    /// Validates persisted account fields.
    pub fn validate(&self) -> Result<()> {
        Self::auth(
            self.name,
            self.manager_authority,
            self.record_authority,
            self.max_epoch_entries,
            self.commission_bps,
            self.commission_account,
            MAX_COMMISSION_BPS,
        )?;

        if self.validator_vote == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }
}
/// Stores claim status for a given leaf in the Merkle tree.
#[account]
#[derive(Default)]
pub struct ClaimStatus {
    /// Whether the claim was already made.
    pub is_claimed: bool,
    /// Who made the claim.
    pub claimant: Pubkey,
    /// Payer of the claim status account.
    pub claim_status_payer: Pubkey,
    /// Slot when the claim was made.
    pub slot_claimed_at: u64,
    /// Amount claimed.
    pub amount: u64,
    /// Expiry of this claim.
    pub expires_at: u64,
    /// PDA bump.
    pub bump: u8,
}

impl ClaimStatus {
    /// PDA seed for claim status accounts.
    pub const SEED: &'static [u8] = b"CLAIM_STATUS";
    /// Account size for rent-exemption.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spendable_lamports_subtracts_rent() {
        assert_eq!(
            RewardCollectionAccount::spendable_lamports(1_000, 100).unwrap(),
            900
        );
        assert!(RewardCollectionAccount::spendable_lamports(50, 100).is_err());
    }

    #[test]
    fn partner_ledger_add_accumulates_same_epoch() {
        let mut ledger = PartnerShareLedger::default();
        ledger.add(5, 100, 4, false).unwrap();
        ledger.add(5, 50, 4, false).unwrap();
        assert_eq!(ledger.entries.len(), 1);
        assert_eq!(ledger.entries[0].amount, 150);
    }

    #[test]
    fn partner_ledger_fifo_evicts_oldest_epoch() {
        let mut ledger = PartnerShareLedger::default();
        for epoch in [1u64, 2, 3, 4] {
            ledger.add(epoch, 10, 4, false).unwrap();
        }
        ledger.add(5, 99, 4, false).unwrap();
        assert_eq!(ledger.entries.len(), 4);
        assert!(!ledger.entries.iter().any(|e| e.epoch == 1));
        assert!(ledger
            .entries
            .iter()
            .any(|e| e.epoch == 5 && e.amount == 99));
    }

    #[test]
    fn tip_backrun_manager_required_when_unset() {
        let config = RewardDistributionConfigAccount::default();
        assert!(config.require_tip_backrun_manager_authority().is_err());
    }

    #[test]
    fn tip_backrun_manager_returns_pubkey_when_set() {
        let key = Pubkey::new_unique();
        let config = RewardDistributionConfigAccount {
            tip_backrun_manager_authority: Some(key),
            ..Default::default()
        };
        assert_eq!(config.require_tip_backrun_manager_authority().unwrap(), key);
    }

    #[test]
    fn partner_share_pda_differs_by_kind_for_same_name_and_vote() {
        let mut name = [0u8; 32];
        name[..7].copy_from_slice(b"Rakurai");
        let vote = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let tip_seeds = PartnerShareAccount::pda_seeds(PartnerShareKind::Tip, &name, &vote);
        let backrun_seeds =
            PartnerShareAccount::pda_seeds(PartnerShareKind::Backrun, &name, &vote);

        let tip_addr = Pubkey::find_program_address(&tip_seeds, &program).0;
        let backrun_addr = Pubkey::find_program_address(&backrun_seeds, &program).0;
        assert_ne!(tip_addr, backrun_addr);
    }

    #[test]
    fn partner_commission_rejects_bps_above_cap() {
        assert!(validate_partner_commission(10_001, Pubkey::new_unique(), 10_000).is_err());
    }

    #[test]
    fn partner_commission_requires_account_when_bps_positive() {
        assert!(validate_partner_commission(100, Pubkey::default(), 10_000).is_err());
        assert!(validate_partner_commission(0, Pubkey::default(), 10_000).is_ok());
    }
}
