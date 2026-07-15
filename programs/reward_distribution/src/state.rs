use crate::ErrorCode::{
    AccountValidationFailure, ArithmeticError, EpochAlreadyClaimed,
    EpochAlreadyConvertedToBlockReward, EpochEntryNotFound, EpochNotClaimed,
    InvalidRevenueEpochCapacity, InvalidRevenueName, InvalidRevenueShareLayout,
    MaxCommissionFeeBpsExceeded, RevenueManagerNotConfigured, RevenueLedgerFull,
};
use anchor_lang::prelude::*;
use std::mem::size_of;

const HEADER_SIZE: usize = 8;

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
    /// If enabled, Client will also deduct its commission from the validator’s MEV commission.
    pub client_commission_on_mev_commission_enabled: Option<bool>,
    /// Authority that may create tip/mev-share revenue share accounts and manage claims. `None` disables revenue share account creation.
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
    pub client_commission_bps: u16,
    /// Account receiving client commission.
    pub client_commission_account: Pubkey,
    /// Epoch when claims expire.
    pub expires_at: u64,
    /// Who initialized the account (validator identity).
    pub initializer: Pubkey,
    /// PDA bump.
    pub bump: u8,
    /// Amount of MEV commission deducted by Client (if enabled).
    pub client_mev_commission_deducted: Option<u64>,
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

/// Rakurai label for tip/mev revenue share vaults (`name` field in PDA seeds; lowercase padded).
pub const RAKURAI_REVENUE_NAME: [u8; 32] = {
    let mut name = [0u8; 32];
    name[0] = b'r';
    name[1] = b'a';
    name[2] = b'k';
    name[3] = b'u';
    name[4] = b'r';
    name[5] = b'a';
    name[6] = b'i';
    name
};

/// Singleton defaults for tip and mev-share revenue share account initialization (`init_v1`).
#[account]
#[derive(Default)]
pub struct TipsAndMevShareConfigAccount {
    /// Authorized updater of this config.
    pub authority: Pubkey,
    /// PDA bump.
    pub bump: u8,

    /// Tip defaults (copied onto TCA at `initialize_revenue_share_account_v1`).
    pub tip_manager_authority: Pubkey,
    pub tip_commission_account: Pubkey,
    pub tip_commission_bps: u16,
    /// Ledger capacity written to `RevenueShareAccount.max_epoch_entries` (1..=32).
    pub tip_epoch: u8,

    /// MevShare defaults (copied onto MCA at `initialize_revenue_share_account_v1`).
    pub mev_share_manager_authority: Pubkey,
    pub mev_share_commission_account: Pubkey,
    pub mev_share_commission_bps: u16,
    /// Ledger capacity written to `RevenueShareAccount.max_epoch_entries` (1..=32).
    pub mev_share_epoch: u8,
}

/// Per-epoch attributed amount.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct EpochAmountEntry {
    pub epoch: u64,
    /// Accumulated via `record_revenue`.
    pub amount: u64,
    /// Settled SOL credited via `settle_revenue`, or auto-credited on `record_revenue` for Rakurai tip TCA.
    pub transferred_amount: u64,
    pub claimed: bool,
    /// Whether this epoch's block reward conversion is complete.
    pub block_reward_converted: bool,
}

/// Pre-`transferred_amount` epoch entry (legacy revenue-share account layout).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct LegacyEpochAmountEntry {
    pub epoch: u64,
    pub amount: u64,
    pub claimed: bool,
    pub block_reward_converted: bool,
}

/// Fixed header bytes after the 8-byte Anchor discriminator, before the ledger vec.
/// `share_kind` through `block_reward_conversion_enabled`.
pub const REVENUE_SHARE_FIXED_PREFIX_LEN: usize = 1  // share_kind
    + 32 // name
    + 32 // validator_vote
    + 32 // initializer
    + 32 // manager_authority
    + 32 // record_authority
    + 1  // max_epoch_entries
    + 2  // commission_bps
    + 32 // commission_account
    + 1; // block_reward_conversion_enabled

/// Byte offset of `max_epoch_entries` within account data (including discriminator).
pub const REVENUE_SHARE_MAX_EPOCH_ENTRIES_OFFSET: usize =
    HEADER_SIZE + 1 + 32 + 32 + 32 + 32 + 32; // after share_kind + name + vote + initializer + manager + record

/// Per-account epoch revenue ledger; grows until `max_epoch_entries` then evicts oldest claimed entry (errors if all unclaimed).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct RevenueLedger {
    pub entries: Vec<EpochAmountEntry>,
}

/// Revenue share vault kind; included in PDA seeds after [`RevenueShareAccount::SEED`].
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RevenueKind {
    Tip,
    MevShare,
}

impl RevenueKind {
    pub const TIP_SEED: &'static [u8] = b"TIP";
    pub const MEV_SHARE_SEED: &'static [u8] = b"MEV_SHARE";

    pub fn seed(self) -> &'static [u8] {
        match self {
            Self::Tip => Self::TIP_SEED,
            Self::MevShare => Self::MEV_SHARE_SEED,
        }
    }
}

/// How the manager adjusts account-level [`RevenueShareAccount::deficit`].
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeficitUpdate {
    /// Replace deficit with `value`.
    Set { value: u64 },
    /// Write off all deficit (`0`).
    Clear,
    /// Add `amount` to deficit.
    Increase { amount: u64 },
    /// Subtract `amount` from deficit (saturating at 0).
    Decrease { amount: u64 },
}

impl DeficitUpdate {
    pub fn apply(self, current: u64) -> Result<u64> {
        use crate::ErrorCode::ArithmeticError;

        Ok(match self {
            Self::Set { value } => value,
            Self::Clear => 0,
            Self::Increase { amount } => current.checked_add(amount).ok_or(ArithmeticError)?,
            Self::Decrease { amount } => current.saturating_sub(amount),
        })
    }
}

/// Tip/mev-share revenue share vault per validator (accounting + lamport vault).
#[account]
pub struct RevenueShareAccount {
    /// Tip vs mev-share; part of the PDA seeds with `name` and `validator_vote`.
    pub share_kind: RevenueKind,
    /// UTF-8 padded label (used in PDA seeds).
    pub name: [u8; 32],
    pub validator_vote: Pubkey,
    /// Who paid to create this account; receives rent on close.
    pub initializer: Pubkey,
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
    /// When true, epoch entries require explicit block reward conversion via `update_epoch_converted_to_block_reward`.
    pub block_reward_conversion_enabled: bool,
    pub ledger: RevenueLedger,
    /// Cumulative unpaid shortfall (`record amount - transferred` when claimed underfunded); manager can write off via `update_deficit`.
    pub deficit: u64,
    pub bump: u8,
}

/// Tips Collection Account (TCA): a [`RevenueShareAccount`] with `share_kind = Tip`.
/// Collects the validator's tip revenue (settled from a custom tip account).
pub type TipsCollectionAccount = RevenueShareAccount;
/// Mev Share Collection Account (MCA): a [`RevenueShareAccount`] with `share_kind = MevShare`.
/// Collects the agreed MEV / arbitrage revenue share.
pub type MevShareCollectionAccount = RevenueShareAccount;

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
        self.client_commission_on_mev_commission_enabled
            .unwrap_or(false)
    }

    /// Sets MEV commission enabled status.
    pub fn set_mev_commission_enabled(&mut self, enabled: bool) {
        self.client_commission_on_mev_commission_enabled = Some(enabled);
    }

    /// Returns the configured tip/mev-share revenue manager, if revenue share account creation is enabled.
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

impl TipsAndMevShareConfigAccount {
    /// PDA seed for the tips-and-mev-share config singleton.
    pub const SEED: &'static [u8] = b"TIPS_AND_MEV_SHARE_CONFIG";
    /// Account size for rent-exemption.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    /// Returns `(manager, commission_account, commission_bps, epoch)` for `share_kind`.
    /// `record_authority` is passed as an instruction arg (same as legacy init).
    pub fn defaults_for(&self, share_kind: RevenueKind) -> (Pubkey, Pubkey, u16, u8) {
        match share_kind {
            RevenueKind::Tip => (
                self.tip_manager_authority,
                self.tip_commission_account,
                self.tip_commission_bps,
                self.tip_epoch,
            ),
            RevenueKind::MevShare => (
                self.mev_share_manager_authority,
                self.mev_share_commission_account,
                self.mev_share_commission_bps,
                self.mev_share_epoch,
            ),
        }
    }

    /// Rent space for a revenue share vault initialized from this config for `share_kind`.
    pub fn space_for_share_kind(&self, share_kind: RevenueKind) -> usize {
        let max_epoch_entries = match share_kind {
            RevenueKind::Tip => self.tip_epoch,
            RevenueKind::MevShare => self.mev_share_epoch,
        };
        RevenueShareAccount::space_for(max_epoch_entries as usize)
    }

    /// Validates tip and mev-share field groups (commission, epoch capacity, authorities).
    pub fn validate(&self) -> Result<()> {
        if self.authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        Self::validate_side(
            self.tip_manager_authority,
            self.tip_commission_account,
            self.tip_commission_bps,
            self.tip_epoch,
        )?;
        Self::validate_side(
            self.mev_share_manager_authority,
            self.mev_share_commission_account,
            self.mev_share_commission_bps,
            self.mev_share_epoch,
        )?;

        Ok(())
    }

    fn validate_side(
        manager_authority: Pubkey,
        commission_account: Pubkey,
        commission_bps: u16,
        epoch: u8,
    ) -> Result<()> {
        if manager_authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }

        if epoch == 0 || epoch as usize > MAX_REVENUE_EPOCH_ENTRIES_CAP {
            return Err(InvalidRevenueEpochCapacity.into());
        }

        validate_commission(commission_bps, commission_account, MAX_COMMISSION_BPS)
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
            || self.client_commission_account == default_pubkey
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
        block_reward_converted: bool,
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        for entry in &mut self.entries {
            if entry.epoch == epoch {
                entry.amount = entry.amount.checked_add(amount).ok_or(ArithmeticError)?;
                return Ok(());
            }
        }

        let new_entry = EpochAmountEntry {
            epoch,
            amount,
            transferred_amount: 0,
            claimed: false,
            block_reward_converted,
        };

        if self.entries.len() < capacity {
            self.entries.push(new_entry);
            return Ok(());
        }

        let mut oldest_claimed_idx: Option<usize> = None;
        let mut oldest_claimed_epoch = u64::MAX;
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.claimed && entry.epoch < oldest_claimed_epoch {
                oldest_claimed_epoch = entry.epoch;
                oldest_claimed_idx = Some(i);
            }
        }

        let evict_idx = oldest_claimed_idx.ok_or(RevenueLedgerFull)?;
        self.entries[evict_idx] = new_entry;
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
            + REVENUE_SHARE_FIXED_PREFIX_LEN
            + 4 // vec length (u32)
            + max_epoch_entries * size_of::<EpochAmountEntry>() // ledger size
            + 8 // deficit
            + 1 // bump
    }

    /// Allocated size for pre-`transferred_amount` / pre-`deficit` revenue-share accounts.
    pub fn space_for_legacy(max_epoch_entries: usize) -> usize {
        HEADER_SIZE
            + REVENUE_SHARE_FIXED_PREFIX_LEN
            + 4 // vec length (u32)
            + max_epoch_entries * size_of::<LegacyEpochAmountEntry>()
            + 1 // bump (no deficit)
    }

    /// Returns true when account data length matches the current (new) layout for its `max_epoch_entries`.
    pub fn is_new_layout(data: &[u8]) -> bool {
        if data.len() <= REVENUE_SHARE_MAX_EPOCH_ENTRIES_OFFSET {
            return false;
        }
        let max_epoch_entries = data[REVENUE_SHARE_MAX_EPOCH_ENTRIES_OFFSET] as usize;
        data.len() == Self::space_for(max_epoch_entries)
    }

    /// Returns true when account data length matches the legacy layout for its `max_epoch_entries`.
    pub fn is_legacy_layout(data: &[u8]) -> bool {
        if data.len() <= REVENUE_SHARE_MAX_EPOCH_ENTRIES_OFFSET {
            return false;
        }
        let max_epoch_entries = data[REVENUE_SHARE_MAX_EPOCH_ENTRIES_OFFSET] as usize;
        data.len() == Self::space_for_legacy(max_epoch_entries)
    }

    /// Reads `manager_authority` and `initializer` from a legacy-layout account (size-validated).
    pub fn read_legacy_close_authorities(data: &[u8]) -> Result<(Pubkey, Pubkey)> {
        if !Self::is_legacy_layout(data) {
            return Err(InvalidRevenueShareLayout.into());
        }
        // Discriminator (8) + share_kind (1) + name (32) + validator_vote (32) = 73
        let initializer_offset = HEADER_SIZE + 1 + 32 + 32;
        let manager_offset = initializer_offset + 32;
        let initializer = Pubkey::new_from_array(
            data[initializer_offset..initializer_offset + 32]
                .try_into()
                .map_err(|_| AccountValidationFailure)?,
        );
        let manager_authority = Pubkey::new_from_array(
            data[manager_offset..manager_offset + 32]
                .try_into()
                .map_err(|_| AccountValidationFailure)?,
        );
        Ok((manager_authority, initializer))
    }

    pub fn populate_on_init(
        &mut self,
        share_kind: RevenueKind,
        name: [u8; 32],
        validator_vote: Pubkey,
        initializer: Pubkey,
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
        self.initializer = initializer;
        self.manager_authority = manager_authority;
        self.record_authority = record_authority;
        self.max_epoch_entries = max_epoch_entries;
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.block_reward_conversion_enabled = false;
        self.ledger = RevenueLedger::default();
        self.deficit = 0;
        self.bump = bump;
        self.validate()
    }

    /// Records attributed revenue for an epoch.
    /// Rakurai tip TCA (`Tip` + `RAKURAI_REVENUE_NAME`): also `saturating_add`s `transferred_amount`
    /// (tip-manager deposits SOL in the same drain tx).
    /// Non-Rakurai: only updates `amount`; callers must use `settle_revenue` (CPI transfer + credit).
    pub fn record_revenue(&mut self, epoch: u64, amount: u64) -> Result<()> {
        let capacity = self.max_epoch_entries as usize;
        self.ledger
            .add(epoch, amount, capacity, !self.block_reward_conversion_enabled)?;

        if self.share_kind == RevenueKind::Tip && self.name == RAKURAI_REVENUE_NAME {
            let entry = self.ledger.get_mut(epoch)?;
            entry.transferred_amount = entry.transferred_amount.saturating_add(amount);
        }
        Ok(())
    }

    /// Credits `transferred_amount` after a settle CPI transfer (non-Rakurai path).
    /// May exceed recorded `amount` (over-settle); claim pays `transferred_amount`.
    pub fn credit_transferred(&mut self, epoch: u64, amount: u64) -> Result<()> {
        use crate::ErrorCode::*;

        if amount == 0 {
            return Err(RewardsTooLow.into());
        }

        let entry = self.ledger.get_mut(epoch)?;
        if entry.claimed {
            return Err(EpochAlreadyClaimed.into());
        }

        entry.transferred_amount = entry
            .transferred_amount
            .checked_add(amount)
            .ok_or(ArithmeticError)?;
        Ok(())
    }

    pub fn is_rakurai_tip_tca(&self) -> bool {
        self.share_kind == RevenueKind::Tip && self.name == RAKURAI_REVENUE_NAME
    }

    /// Marks a claimed epoch entry as converted to block rewards.
    /// Requires entry `claimed` and entry flag still false.
    pub fn mark_epoch_converted_to_block_reward(&mut self, epoch: u64) -> Result<()> {
        let entry = self.ledger.get_mut(epoch)?;
        if !entry.claimed {
            return Err(EpochNotClaimed.into());
        }
        if entry.block_reward_converted {
            return Err(EpochAlreadyConvertedToBlockReward.into());
        }

        entry.block_reward_converted = true;
        Ok(())
    }

    pub fn update_commission(
        &mut self,
        commission_bps: u16,
        commission_account: Pubkey,
        block_reward_conversion_enabled: bool,
        manager_authority: Pubkey,
        record_authority: Option<Pubkey>,
    ) -> Result<()> {
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.block_reward_conversion_enabled = block_reward_conversion_enabled;
        self.manager_authority = manager_authority;
        if let Some(new_record_authority) = record_authority {
            self.record_authority = new_record_authority;
        }
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

        if self.manager_authority == Pubkey::default()
            || self.validator_vote == Pubkey::default()
            || self.initializer == Pubkey::default()
        {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    /// Splits claimable settled funds for an epoch between commission and validator identity.
    /// Claimable = `transferred_amount` (may be above or below recorded `amount`).
    /// If underfunded (`transferred < amount`), adds shortfall to `deficit`.
    /// Returns `(commission, validator, claimed_total, shortfall)`.
    pub fn claim_revenue(
        ledger: &mut RevenueLedger,
        deficit: &mut u64,
        revenue_share_account: AccountInfo,
        commission_account: AccountInfo,
        validator_identity: AccountInfo,
        commission_bps: u16,
        epoch: u64,
    ) -> Result<(u64, u64, u64, u64)> {
        use crate::ErrorCode::*;

        let current_epoch = Clock::get()?.epoch;
        if current_epoch <= epoch {
            return Err(PrematureRevenueClaim.into());
        }

        let (record_amount, transferred_amount) = {
            let entry = ledger
                .entries
                .iter()
                .find(|e| e.epoch == epoch)
                .ok_or(EpochEntryNotFound)?;
            if entry.claimed {
                return Err(EpochAlreadyClaimed.into());
            }
            if entry.amount == 0 && entry.transferred_amount == 0 {
                return Err(RewardsTooLow.into());
            }
            (entry.amount, entry.transferred_amount)
        };

        // Always pay what was transferred (supports under-settle and over-settle).
        let claimable = transferred_amount;
        if claimable == 0 {
            return Err(RewardsTooLow.into());
        }

        let shortfall = record_amount.saturating_sub(claimable);

        let commission_amount = if commission_bps == 0 {
            0
        } else {
            claimable
                .checked_mul(commission_bps as u64)
                .ok_or(ArithmeticError)?
                .checked_div(10_000)
                .ok_or(ArithmeticError)?
        };
        let validator_amount = claimable
            .checked_sub(commission_amount)
            .ok_or(ArithmeticError)?;

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(revenue_share_account.data_len());
        let available = revenue_share_account.lamports().saturating_sub(min_rent);
        if available < claimable {
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

        if shortfall > 0 {
            *deficit = deficit.checked_add(shortfall).ok_or(ArithmeticError)?;
        }

        ledger.mark_claimed(epoch)?;

        Ok((commission_amount, validator_amount, claimable, shortfall))
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
    fn revenue_ledger_evicts_oldest_claimed_epoch() {
        let mut ledger = RevenueLedger::default();
        for epoch in [1u64, 2, 3, 4] {
            ledger.add(epoch, 10, 4, false).unwrap();
        }
        for epoch in [1u64, 2, 3, 4] {
            ledger.mark_claimed(epoch).unwrap();
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
    fn revenue_ledger_skips_unclaimed_when_evicting() {
        let mut ledger = RevenueLedger::default();
        for epoch in [1u64, 2, 3, 4] {
            ledger.add(epoch, 10, 4, false).unwrap();
        }
        // claim epochs 2, 3, 4 but leave 1 unclaimed
        for epoch in [2u64, 3, 4] {
            ledger.mark_claimed(epoch).unwrap();
        }
        ledger.add(5, 99, 4, false).unwrap();
        assert_eq!(ledger.entries.len(), 4);
        // epoch 1 (unclaimed) survives; epoch 2 (oldest claimed) is evicted
        assert!(ledger.entries.iter().any(|e| e.epoch == 1 && !e.claimed));
        assert!(!ledger.entries.iter().any(|e| e.epoch == 2));
        assert!(ledger.entries.iter().any(|e| e.epoch == 5));
    }

    #[test]
    fn revenue_ledger_errors_when_all_unclaimed() {
        let mut ledger = RevenueLedger::default();
        for epoch in [1u64, 2, 3, 4] {
            ledger.add(epoch, 10, 4, false).unwrap();
        }
        assert!(ledger.add(5, 99, 4, false).is_err());
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
        let mev_share_seeds = RevenueShareAccount::pda_seeds(RevenueKind::MevShare, &name, &vote);

        let tip_addr = Pubkey::find_program_address(&tip_seeds, &program).0;
        let mev_share_addr = Pubkey::find_program_address(&mev_share_seeds, &program).0;
        assert_ne!(tip_addr, mev_share_addr);
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

    fn valid_tips_and_mev_share_config() -> TipsAndMevShareConfigAccount {
        TipsAndMevShareConfigAccount {
            authority: Pubkey::new_unique(),
            bump: 255,
            tip_manager_authority: Pubkey::new_unique(),
            tip_commission_account: Pubkey::new_unique(),
            tip_commission_bps: 500,
            tip_epoch: 8,
            mev_share_manager_authority: Pubkey::new_unique(),
            mev_share_commission_account: Pubkey::new_unique(),
            mev_share_commission_bps: 1_000,
            mev_share_epoch: 16,
        }
    }

    #[test]
    fn tips_and_mev_share_config_validate_ok() {
        assert!(valid_tips_and_mev_share_config().validate().is_ok());
    }

    #[test]
    fn tips_and_mev_share_config_rejects_bad_epoch() {
        let mut cfg = valid_tips_and_mev_share_config();
        cfg.tip_epoch = 0;
        assert!(cfg.validate().is_err());
        cfg.tip_epoch = 8;
        cfg.mev_share_epoch = 33;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn tips_and_mev_share_config_defaults_for_selects_field_group() {
        let cfg = valid_tips_and_mev_share_config();
        let (tip_mgr, tip_comm_acc, tip_bps, tip_ep) = cfg.defaults_for(RevenueKind::Tip);
        assert_eq!(tip_mgr, cfg.tip_manager_authority);
        assert_eq!(tip_comm_acc, cfg.tip_commission_account);
        assert_eq!(tip_bps, cfg.tip_commission_bps);
        assert_eq!(tip_ep, cfg.tip_epoch);

        let (mev_mgr, mev_comm_acc, mev_bps, mev_ep) = cfg.defaults_for(RevenueKind::MevShare);
        assert_eq!(mev_mgr, cfg.mev_share_manager_authority);
        assert_eq!(mev_comm_acc, cfg.mev_share_commission_account);
        assert_eq!(mev_bps, cfg.mev_share_commission_bps);
        assert_eq!(mev_ep, cfg.mev_share_epoch);
    }

    #[test]
    fn revenue_share_layout_size_helpers_differ() {
        assert_ne!(
            RevenueShareAccount::space_for(8),
            RevenueShareAccount::space_for_legacy(8)
        );
        assert!(RevenueShareAccount::space_for(8) > RevenueShareAccount::space_for_legacy(8));
    }

    #[test]
    fn deficit_update_apply_variants() {
        assert_eq!(DeficitUpdate::Set { value: 42 }.apply(7).unwrap(), 42);
        assert_eq!(DeficitUpdate::Clear.apply(100).unwrap(), 0);
        assert_eq!(
            DeficitUpdate::Increase { amount: 5 }.apply(10).unwrap(),
            15
        );
        assert_eq!(
            DeficitUpdate::Decrease { amount: 3 }.apply(10).unwrap(),
            7
        );
        assert_eq!(
            DeficitUpdate::Decrease { amount: 99 }.apply(10).unwrap(),
            0
        );
    }

    #[test]
    fn credit_transferred_allows_over_settle() {
        let mut acc = test_revenue_share_account(false);
        acc.record_revenue(10, 100).unwrap();
        acc.credit_transferred(10, 40).unwrap();
        acc.credit_transferred(10, 80).unwrap(); // 120 > recorded 100
        assert_eq!(acc.ledger.entries[0].transferred_amount, 120);
    }

    #[test]
    fn record_revenue_rakurai_tip_also_credits_transferred() {
        let mut acc = test_revenue_share_account(false);
        acc.share_kind = RevenueKind::Tip;
        acc.name = RAKURAI_REVENUE_NAME;
        acc.record_revenue(10, 100).unwrap();
        assert_eq!(acc.ledger.entries[0].amount, 100);
        assert_eq!(acc.ledger.entries[0].transferred_amount, 100);
        acc.record_revenue(10, 25).unwrap();
        assert_eq!(acc.ledger.entries[0].amount, 125);
        assert_eq!(acc.ledger.entries[0].transferred_amount, 125);
    }

    #[test]
    fn record_revenue_non_rakurai_does_not_auto_credit_transferred() {
        let mut acc = test_revenue_share_account(false);
        acc.record_revenue(10, 100).unwrap();
        assert_eq!(acc.ledger.entries[0].transferred_amount, 0);
    }

    fn test_revenue_share_account(block_reward_conversion_enabled: bool) -> RevenueShareAccount {
        let mut name = [0u8; 32];
        name[0] = b'X';
        RevenueShareAccount {
            share_kind: RevenueKind::Tip,
            name,
            validator_vote: Pubkey::new_unique(),
            initializer: Pubkey::new_unique(),
            manager_authority: Pubkey::new_unique(),
            record_authority: Pubkey::new_unique(),
            max_epoch_entries: 4,
            commission_bps: 0,
            commission_account: Pubkey::default(),
            block_reward_conversion_enabled,
            ledger: RevenueLedger::default(),
            deficit: 0,
            bump: 0,
        }
    }

    #[test]
    fn record_revenue_inits_entry_false_when_convert_flag_true() {
        let mut acc = test_revenue_share_account(true);
        acc.record_revenue(10, 100).unwrap();
        assert!(!acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn record_revenue_inits_entry_true_when_convert_flag_false() {
        let mut acc = test_revenue_share_account(false);
        acc.record_revenue(10, 100).unwrap();
        assert!(acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn record_revenue_accumulate_does_not_overwrite_converted_flag() {
        let mut acc = test_revenue_share_account(true);
        acc.record_revenue(10, 100).unwrap();
        assert!(!acc.ledger.entries[0].block_reward_converted);
        acc.ledger.entries[0].block_reward_converted = true;
        acc.record_revenue(10, 50).unwrap();
        assert!(acc.ledger.entries[0].block_reward_converted);
        assert_eq!(acc.ledger.entries[0].amount, 150);
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_sets_flag_when_claimed() {
        let mut acc = test_revenue_share_account(true);
        acc.record_revenue(10, 100).unwrap();
        acc.ledger.mark_claimed(10).unwrap();
        acc.mark_epoch_converted_to_block_reward(10).unwrap();
        assert!(acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_works_regardless_of_account_flag() {
        let mut acc = test_revenue_share_account(false);
        acc.ledger.add(10, 100, 4, false).unwrap();
        acc.ledger.mark_claimed(10).unwrap();
        acc.mark_epoch_converted_to_block_reward(10).unwrap();
        assert!(acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn mark_epoch_converted_to_block_reward_requires_claimed() {
        let mut acc = test_revenue_share_account(true);
        acc.record_revenue(10, 100).unwrap();
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
