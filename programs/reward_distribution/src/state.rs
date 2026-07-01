use crate::ErrorCode::{
    AccountValidationFailure, ArithmeticError, ConvertToBlockRewardsNotEnabled,
    EpochAlreadyClaimed, EpochEntryNotFound, EpochNotClaimed, InvalidRevenueEpochCapacity,
    InvalidRevenueName, MaxCommissionFeeBpsExceeded, RevenueManagerNotConfigured,
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
    /// Authority that may create tip/backrun revenue share accounts and manage claims. `None` disables revenue share account creation.
    pub revenue_manager_authority: Option<Pubkey>,
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

pub const MAX_REVENUE_EPOCH_ENTRIES_CAP: usize = 32;

/// Per-epoch attributed amount.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct EpochAmountEntry {
    pub epoch: u64,
    pub amount: u64,
    pub claimed: bool,
    /// When true, this epoch's share is converted to block rewards on claim.
    pub converted_to_block_reward: bool,
}

/// Per-account epoch revenue ledger; grows until `max_epoch_entries` then overrides oldest epoch.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct RevenueLedger {
    pub entries: Vec<EpochAmountEntry>,
}

/// Revenue share vault kind; included in PDA seeds after [`RevenueShareAccount::SEED`].
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RevenueKind {
    Tip,
    Backrun,
}

impl RevenueKind {
    pub const TIP_SEED: &'static [u8] = b"TIP";
    pub const BACKRUN_SEED: &'static [u8] = b"BACKRUN";

    pub fn seed(self) -> &'static [u8] {
        match self {
            Self::Tip => Self::TIP_SEED,
            Self::Backrun => Self::BACKRUN_SEED,
        }
    }
}

/// Tip/backrun revenue share vault per validator (accounting + lamport vault).
#[account]
pub struct RevenueShareAccount {
    /// Tip vs backrun; part of the PDA seeds with `name` and `validator_vote`.
    pub share_kind: RevenueKind,
    /// UTF-8 padded label (used in PDA seeds).
    pub name: [u8; 32],
    pub validator_vote: Pubkey,
    /// Claims revenue, updates config, and closes the account.
    pub manager_authority: Pubkey,
    /// Signs `record_revenue`.
    pub record_authority: Pubkey,
    /// Max distinct epochs in `ledger`.
    pub max_epoch_entries: u8,
    /// Commission on revenue claims (basis points); remainder goes to validator identity.
    pub commission_bps: u16,
    /// Receives the commission portion on claim.
    pub commission_account: Pubkey,
    /// When true, recorded share is converted to block rewards on claim.
    pub convert_to_block_rewards: bool,
    pub ledger: RevenueLedger,
    pub bump: u8,
}

/// Tips Collection Account (TCA): a [`RevenueShareAccount`] with `share_kind = Tip`.
/// Collects the validator's tip revenue (settled from a custom tip account).
pub type TipsCollectionAccount = RevenueShareAccount;
/// Backrun Collection Account (BCA): a [`RevenueShareAccount`] with `share_kind = Backrun`.
/// Collects the agreed backrun/arbitrage revenue.
pub type BackrunCollectionAccount = RevenueShareAccount;

const HEADER_SIZE: usize = 8;
const MAX_COMMISSION_BPS: u16 = 10000;

/// Validates revenue commission fields against config caps.
pub fn validate_commission(
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

    /// Returns the configured tip/backrun revenue manager, if revenue share account creation is enabled.
    pub fn require_revenue_manager_authority(&self) -> Result<Pubkey> {
        self.revenue_manager_authority
            .filter(|key| *key != Pubkey::default())
            .ok_or(RevenueManagerNotConfigured.into())
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

impl RevenueLedger {
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

impl RevenueShareAccount {
    pub const SEED: &'static [u8] = b"REVENUE_SHARE";

    pub fn pda_seeds<'a>(
        share_kind: RevenueKind,
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
            + 1  //share_kind
            + 32 // name
            + 32 // validator_vote
            + 32 // manager_authority
            + 32 // record_authority
            + 1 // max_epoch_entries
            + 2 // commission_bps
            + 32 // commission_account
            + 1 // convert_to_block_rewards
            + 4 // vec length (u32)
            + max_epoch_entries * size_of::<EpochAmountEntry>() // ledger size
            + 1 // bump
    }

    pub fn populate_on_init(
        &mut self,
        share_kind: RevenueKind,
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
        self.ledger = RevenueLedger::default();
        self.bump = bump;
        self.validate()
    }

    pub fn record_revenue(&mut self, epoch: u64, amount: u64) -> Result<()> {
        let capacity = self.max_epoch_entries as usize;
        self.ledger
            .add(epoch, amount, capacity, self.convert_to_block_rewards)
    }

    /// Marks a claimed epoch entry as converted to block rewards.
    /// Requires account `convert_to_block_rewards`, entry `claimed`, and entry flag still false.
    pub fn mark_epoch_converted_to_block_reward(&mut self, epoch: u64) -> Result<()> {
        if !self.convert_to_block_rewards {
            return Err(ConvertToBlockRewardsNotEnabled.into());
        }

        let entry = self.ledger.get_mut(epoch)?;
        if !entry.claimed {
            return Err(EpochNotClaimed.into());
        }
        if entry.converted_to_block_reward {
            return Ok(());
        }

        entry.converted_to_block_reward = true;
        Ok(())
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
            return Err(InvalidRevenueName.into());
        }

        if record_authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        if max_epoch_entries == 0 || max_epoch_entries as usize > MAX_REVENUE_EPOCH_ENTRIES_CAP {
            return Err(InvalidRevenueEpochCapacity.into());
        }

        validate_commission(commission_bps, commission_account, max_commission_bps)?;

        Ok(())
    }

    /// Validates persisted account fields.
    pub fn validate(&self) -> Result<()> {
        Self::validate_init_params(
            self.name,
            self.record_authority,
            self.max_epoch_entries,
            self.commission_bps,
            self.commission_account,
            MAX_COMMISSION_BPS,
        )?;

        if self.manager_authority == Pubkey::default() || self.validator_vote == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    /// Splits a claimable epoch entry between the commission account and validator identity,
    /// transferring lamports out of the revenue share vault. Returns `(commission, validator)`.
    pub fn claim_revenue(
        ledger: &mut RevenueLedger,
        revenue_share_account: AccountInfo,
        commission_account: AccountInfo,
        validator_identity: AccountInfo,
        commission_bps: u16,
        epoch: u64,
    ) -> Result<(u64, u64)> {
        use crate::ErrorCode::*;

        let current_epoch = Clock::get()?.epoch;
        if current_epoch <= epoch {
            return Err(PrematureRevenueClaim.into());
        }

        let entry_amount = {
            let entry = ledger
                .entries
                .iter()
                .find(|e| e.epoch == epoch)
                .ok_or(EpochEntryNotFound)?;
            if entry.claimed {
                return Err(EpochAlreadyClaimed.into());
            }
            if entry.amount == 0 {
                return Err(RewardsTooLow.into());
            }
            entry.amount
        };

        let commission_amount = if commission_bps == 0 {
            0
        } else {
            entry_amount
                .checked_mul(commission_bps as u64)
                .ok_or(ArithmeticError)?
                .checked_div(10_000)
                .ok_or(ArithmeticError)?
        };
        let validator_amount = entry_amount
            .checked_sub(commission_amount)
            .ok_or(ArithmeticError)?;

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(revenue_share_account.data_len());
        let available = revenue_share_account.lamports().saturating_sub(min_rent);
        if available < entry_amount {
            return Err(RewardsTooLow.into());
        }

        if commission_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                revenue_share_account.clone(),
                commission_account,
                commission_amount,
            )?;
        }
        if validator_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                revenue_share_account,
                validator_identity,
                validator_amount,
            )?;
        }

        ledger.mark_claimed(epoch)?;

        Ok((commission_amount, validator_amount))
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
    fn revenue_ledger_add_accumulates_same_epoch() {
        let mut ledger = RevenueLedger::default();
        ledger.add(5, 100, 4, false).unwrap();
        ledger.add(5, 50, 4, false).unwrap();
        assert_eq!(ledger.entries.len(), 1);
        assert_eq!(ledger.entries[0].amount, 150);
    }

    #[test]
    fn revenue_ledger_fifo_evicts_oldest_epoch() {
        let mut ledger = RevenueLedger::default();
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
    fn revenue_manager_required_when_unset() {
        let config = RewardDistributionConfigAccount::default();
        assert!(config.require_revenue_manager_authority().is_err());
    }

    #[test]
    fn revenue_manager_returns_pubkey_when_set() {
        let key = Pubkey::new_unique();
        let config = RewardDistributionConfigAccount {
            revenue_manager_authority: Some(key),
            ..Default::default()
        };
        assert_eq!(config.require_revenue_manager_authority().unwrap(), key);
    }

    #[test]
    fn revenue_share_pda_differs_by_kind_for_same_name_and_vote() {
        let mut name = [0u8; 32];
        name[..7].copy_from_slice(b"Rakurai");
        let vote = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let tip_seeds = RevenueShareAccount::pda_seeds(RevenueKind::Tip, &name, &vote);
        let backrun_seeds = RevenueShareAccount::pda_seeds(RevenueKind::Backrun, &name, &vote);

        let tip_addr = Pubkey::find_program_address(&tip_seeds, &program).0;
        let backrun_addr = Pubkey::find_program_address(&backrun_seeds, &program).0;
        assert_ne!(tip_addr, backrun_addr);
    }

    #[test]
    fn commission_rejects_bps_above_cap() {
        assert!(validate_commission(10_001, Pubkey::new_unique(), 10_000).is_err());
    }

    #[test]
    fn commission_requires_account_when_bps_positive() {
        assert!(validate_commission(100, Pubkey::default(), 10_000).is_err());
        assert!(validate_commission(0, Pubkey::default(), 10_000).is_ok());
    }

    fn test_revenue_share_account(convert_to_block_rewards: bool) -> RevenueShareAccount {
        let mut name = [0u8; 32];
        name[0] = b'X';
        RevenueShareAccount {
            share_kind: RevenueKind::Tip,
            name,
            validator_vote: Pubkey::new_unique(),
            manager_authority: Pubkey::new_unique(),
            record_authority: Pubkey::new_unique(),
            max_epoch_entries: 4,
            commission_bps: 0,
            commission_account: Pubkey::default(),
            convert_to_block_rewards,
            ledger: RevenueLedger::default(),
            bump: 0,
        }
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_sets_flag_when_claimed() {
        let mut acc = test_revenue_share_account(true);
        // Entry snapshotted false at record time; account flag is true at mark time.
        acc.ledger.add(10, 100, 4, false).unwrap();
        acc.ledger.mark_claimed(10).unwrap();
        acc.mark_epoch_converted_to_block_reward(10).unwrap();
        assert!(acc.ledger.entries[0].converted_to_block_reward);
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_requires_account_flag() {
        let mut acc = test_revenue_share_account(false);
        acc.ledger.add(10, 100, 4, false).unwrap();
        acc.ledger.mark_claimed(10).unwrap();
        assert!(acc.mark_epoch_converted_to_block_reward(10).is_err());
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_requires_claimed() {
        let mut acc = test_revenue_share_account(true);
        acc.ledger.add(10, 100, 4, true).unwrap();
        assert!(acc.mark_epoch_converted_to_block_reward(10).is_err());
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_rejects_already_converted() {
        let mut acc = test_revenue_share_account(true);
        acc.ledger.add(10, 100, 4, false).unwrap();
        acc.ledger.mark_claimed(10).unwrap();
        acc.mark_epoch_converted_to_block_reward(10).unwrap();
        assert!(acc.mark_epoch_converted_to_block_reward(10).is_err());
    }
}
