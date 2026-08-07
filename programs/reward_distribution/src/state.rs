use crate::ErrorCode::{
    AccountValidationFailure, ArithmeticError, EpochAlreadyClaimed,
    EpochAlreadyConvertedToBlockReward, EpochEntryNotFound, EpochNotClaimed,
    InvalidRevenueEpochCapacity, InvalidRevenueName, MaxCommissionFeeBpsExceeded,
    RevenueLedgerFull, RevenueManagerNotConfigured,
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
    /// Ledger capacity written to `max_epoch_entries` (1..=32).
    pub tip_epoch: u8,

    /// MevShare defaults (copied onto MCA at `initialize_revenue_share_account_v1`).
    pub mev_share_manager_authority: Pubkey,
    pub mev_share_commission_account: Pubkey,
    pub mev_share_commission_bps: u16,
    /// Ledger capacity written to `max_epoch_entries` (1..=32).
    pub mev_share_epoch: u8,
}

/// Legacy per-epoch attributed amount (no `transferred_amount`).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct EpochAmountEntry {
    pub epoch: u64,
    pub amount: u64,
    pub claimed: bool,
    /// Whether this epoch's block reward conversion is complete.
    pub block_reward_converted: bool,
}

/// V1 per-epoch attributed amount with settle tracking.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct EpochAmountEntryV1 {
    pub epoch: u64,
    /// Accumulated via `record_revenue`.
    pub amount: u64,
    /// Settled SOL credited via `settle_revenue`, or auto-credited on `record_revenue` for Rakurai tip TCA.
    pub transferred_amount: u64,
    pub claimed: bool,
    /// Whether this epoch's block reward conversion is complete.
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

/// Legacy per-account epoch revenue ledger.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct RevenueLedger {
    pub entries: Vec<EpochAmountEntry>,
}

/// V1 per-account epoch revenue ledger.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct RevenueLedgerV1 {
    pub entries: Vec<EpochAmountEntryV1>,
}

/// Revenue share vault kind; included in PDA seeds after the vault base seed.
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

/// How the manager adjusts account-level [`RevenueShareAccountV1::deficit`].
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

/// Legacy tip/mev-share revenue share vault (no `transferred_amount` / `deficit`).
/// PDA: `[REVENUE_SHARE, TIP|MEV_SHARE, name, vote]`.
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
    pub bump: u8,
}

/// V1 tip/mev-share revenue share vault with settle tracking and deficit.
/// PDA: `[REVENUE_SHARE_V1, TIP|MEV_SHARE, name, vote]`.
#[account]
pub struct RevenueShareAccountV1 {
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
    pub ledger: RevenueLedgerV1,
    /// Cumulative unpaid shortfall (`record amount - transferred` when claimed underfunded); manager can write off via `update_deficit`.
    pub deficit: u64,
    pub bump: u8,
}

/// Tips Collection Account (TCA): legacy [`RevenueShareAccount`] with `share_kind = Tip`.
pub type TipsCollectionAccount = RevenueShareAccount;
/// Mev Share Collection Account (MCA): legacy [`RevenueShareAccount`] with `share_kind = MevShare`.
pub type MevShareCollectionAccount = RevenueShareAccount;

/// Tips Collection Account V1 (TCAV1): [`RevenueShareAccountV1`] with `share_kind = Tip`.
pub type TipsCollectionAccountV1 = RevenueShareAccountV1;
/// Mev Share Collection Account V1 (MCAV1): [`RevenueShareAccountV1`] with `share_kind = MevShare`.
pub type MevShareCollectionAccountV1 = RevenueShareAccountV1;

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

    /// Rent space for a V1 revenue share vault initialized from this config for `share_kind`.
    pub fn space_for_share_kind(&self, share_kind: RevenueKind) -> usize {
        let max_epoch_entries = match share_kind {
            RevenueKind::Tip => self.tip_epoch,
            RevenueKind::MevShare => self.mev_share_epoch,
        };
        RevenueShareAccountV1::space_for(max_epoch_entries as usize)
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

impl RevenueLedgerV1 {
    pub fn get_mut(&mut self, epoch: u64) -> Result<&mut EpochAmountEntryV1> {
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

        let new_entry = EpochAmountEntryV1 {
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
    /// Legacy vault PDA base seed.
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
            + max_epoch_entries * size_of::<EpochAmountEntry>()
            + 1 // bump (no deficit)
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
        self.bump = bump;
        self.validate()
    }

    /// Records attributed revenue for an epoch (amount only; legacy semantics).
    pub fn record_revenue(&mut self, epoch: u64, amount: u64) -> Result<()> {
        let capacity = self.max_epoch_entries as usize;
        self.ledger.add(
            epoch,
            amount,
            capacity,
            !self.block_reward_conversion_enabled,
        )
    }

    /// Marks a claimed epoch entry as converted to block rewards.
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

    pub fn is_rakurai_tip_tca(&self) -> bool {
        self.share_kind == RevenueKind::Tip && self.name == RAKURAI_REVENUE_NAME
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

    /// Splits a claimable epoch entry between the commission account and validator identity,
    /// transferring lamports out of the revenue share vault. Pays recorded `amount` if vault funded.
    /// Returns `(commission, validator)`.
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

impl RevenueShareAccountV1 {
    /// V1 vault PDA base seed.
    pub const SEED: &'static [u8] = b"REVENUE_SHARE_V1";
    /// Alias for [`Self::SEED`].
    pub const SEED_V1: &'static [u8] = b"REVENUE_SHARE_V1";

    pub fn pda_seeds<'a>(
        share_kind: RevenueKind,
        name: &'a [u8; 32],
        validator_vote: &'a Pubkey,
    ) -> [&'a [u8]; 4] {
        Self::pda_seeds_v1(share_kind, name, validator_vote)
    }

    pub fn pda_seeds_v1<'a>(
        share_kind: RevenueKind,
        name: &'a [u8; 32],
        validator_vote: &'a Pubkey,
    ) -> [&'a [u8]; 4] {
        [
            Self::SEED_V1,
            share_kind.seed(),
            name.as_ref(),
            validator_vote.as_ref(),
        ]
    }

    pub fn space_for(max_epoch_entries: usize) -> usize {
        HEADER_SIZE
            + REVENUE_SHARE_FIXED_PREFIX_LEN
            + 4 // vec length (u32)
            + max_epoch_entries * size_of::<EpochAmountEntryV1>()
            + 8 // deficit
            + 1 // bump
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
        self.ledger = RevenueLedgerV1::default();
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
        self.ledger.add(
            epoch,
            amount,
            capacity,
            !self.block_reward_conversion_enabled,
        )?;

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
        ledger: &mut RevenueLedgerV1,
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

    /// Clear open account-level deficit: balance already on vault is split to commission +
    /// validator (same as claim), then deficit reduced. Partial clear OK.
    /// Returns `(applied, commission, validator)`.
    pub fn clear_deficit(
        deficit: &mut u64,
        vault: AccountInfo,
        commission_account: AccountInfo,
        validator_identity: AccountInfo,
        commission_bps: u16,
        amount: u64,
    ) -> Result<(u64, u64, u64)> {
        use crate::ErrorCode::*;

        if amount == 0 {
            return Err(RewardsTooLow.into());
        }
        if *deficit == 0 {
            return Err(NoDeficit.into());
        }

        let applied = amount.min(*deficit);
        let commission_amount = if commission_bps == 0 {
            0
        } else {
            applied
                .checked_mul(commission_bps as u64)
                .ok_or(ArithmeticError)?
                .checked_div(10_000)
                .ok_or(ArithmeticError)?
        };
        let validator_amount = applied
            .checked_sub(commission_amount)
            .ok_or(ArithmeticError)?;

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(vault.data_len());
        let available = vault.lamports().saturating_sub(min_rent);
        if available < applied {
            return Err(RewardsTooLow.into());
        }

        if commission_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                vault.clone(),
                commission_account,
                commission_amount,
            )?;
        }
        if validator_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                vault,
                validator_identity,
                validator_amount,
            )?;
        }

        *deficit = deficit.saturating_sub(applied);
        Ok((applied, commission_amount, validator_amount))
    }
}

// ---------------------------------------------------------------------------
// P2C prepaid subscription escrow
// ---------------------------------------------------------------------------

/// Fixed header bytes after the 8-byte Anchor discriminator, before the ledger vec.
pub const P2C_SUBSCRIPTION_FIXED_PREFIX_LEN: usize = 32 // name
    + 32 // validator_vote
    + 32 // initializer
    + 32 // manager_authority
    + 32 // record_authority
    + 1  // max_epoch_entries
    + 2  // commission_bps
    + 32 // commission_account
    + 1  // grace_epochs
    + 1  // block_reward_conversion_enabled
    + 1  // unpaid_streak
    + 1  // status
    + 8; // deficit

/// Service eligibility for P2C (clients/ops read this; program does not call out).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum P2CSubscriptionStatus {
    #[default]
    Active = 0,
    /// unpaid_streak in 1..=grace_epochs
    InGrace = 1,
    /// unpaid_streak > grace_epochs
    Suspended = 2,
}

/// Per-epoch prepaid fee row for a P2C subscription escrow.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct P2CEpochEntry {
    pub epoch: u64,
    /// Off-chain stake snapshot used to price this epoch.
    pub stake: u64,
    /// Fee due for the epoch.
    pub amount_due: u64,
    /// Lamports paid out on claim for this epoch (`min(due, free prepaid)`).
    pub amount_deducted: u64,
    /// Whether this epoch has been claimed (payout attempted).
    pub claimed: bool,
    /// Same lifecycle flag as revenue share after claim.
    pub block_reward_converted: bool,
}

/// Epoch fee ledger for a P2C subscription escrow.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct P2CSubscriptionLedger {
    pub entries: Vec<P2CEpochEntry>,
}

/// Prepaid P2C subscription escrow per `(name, validator_vote)`.
/// PDA: `[P2C_SUBSCRIPTION, name, vote]`. Manager-only create.
#[account]
pub struct P2CSubscriptionAccount {
    pub name: [u8; 32],
    pub validator_vote: Pubkey,
    /// Who paid rent; receives residual on close.
    pub initializer: Pubkey,
    /// Creates (via ix signer), claims, config, close.
    pub manager_authority: Pubkey,
    /// Signs `record_p2c_subscription`.
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    /// e.g. 2000 = 20% to `commission_account` on claim.
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    /// Consecutive unpaid finished epochs allowed before Suspended (default 2).
    pub grace_epochs: u8,
    pub block_reward_conversion_enabled: bool,
    pub unpaid_streak: u8,
    pub status: P2CSubscriptionStatus,
    /// Cumulative shortfall closed on underfunded claims.
    pub deficit: u64,
    pub ledger: P2CSubscriptionLedger,
    pub bump: u8,
}

impl P2CSubscriptionLedger {
    pub fn get_mut(&mut self, epoch: u64) -> Result<&mut P2CEpochEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.epoch == epoch)
            .ok_or(EpochEntryNotFound.into())
    }

    pub fn get(&self, epoch: u64) -> Result<&P2CEpochEntry> {
        self.entries
            .iter()
            .find(|e| e.epoch == epoch)
            .ok_or(EpochEntryNotFound.into())
    }

    /// Insert a new epoch row; evict oldest claimed if at capacity.
    pub fn insert(&mut self, entry: P2CEpochEntry, capacity: usize) -> Result<()> {
        if self.entries.iter().any(|e| e.epoch == entry.epoch) {
            return Err(crate::ErrorCode::P2CEpochAlreadyRecorded.into());
        }

        if self.entries.len() < capacity {
            self.entries.push(entry);
            return Ok(());
        }

        let mut oldest_claimed_idx: Option<usize> = None;
        let mut oldest_claimed_epoch = u64::MAX;
        for (i, e) in self.entries.iter().enumerate() {
            if e.claimed && e.epoch < oldest_claimed_epoch {
                oldest_claimed_epoch = e.epoch;
                oldest_claimed_idx = Some(i);
            }
        }

        let evict_idx = oldest_claimed_idx.ok_or(RevenueLedgerFull)?;
        self.entries[evict_idx] = entry;
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

    /// Sum of unclaimed `amount_deducted` (normally 0 — reserved only mid-claim).
    pub fn reserved_unclaimed(&self) -> Result<u64> {
        let mut total = 0u64;
        for e in &self.entries {
            if !e.claimed {
                total = total
                    .checked_add(e.amount_deducted)
                    .ok_or(ArithmeticError)?;
            }
        }
        Ok(total)
    }
}

impl P2CSubscriptionAccount {
    pub const SEED: &'static [u8] = b"P2C_SUBSCRIPTION";
    pub const DEFAULT_GRACE_EPOCHS: u8 = 2;

    pub fn pda_seeds<'a>(name: &'a [u8; 32], validator_vote: &'a Pubkey) -> [&'a [u8]; 3] {
        [Self::SEED, name.as_ref(), validator_vote.as_ref()]
    }

    pub fn space_for(max_epoch_entries: usize) -> usize {
        HEADER_SIZE
            + P2C_SUBSCRIPTION_FIXED_PREFIX_LEN
            + 4 // vec length
            + max_epoch_entries * size_of::<P2CEpochEntry>()
            + 1 // bump
    }

    /// Free prepaid above rent, less unclaimed reservations (pure; no sysvar).
    pub fn free_balance_from_parts(lamports: u64, min_rent: u64, reserved_unclaimed: u64) -> u64 {
        lamports
            .saturating_sub(min_rent)
            .saturating_sub(reserved_unclaimed)
    }

    /// Free prepaid balance: lamports above rent, less unclaimed deductions.
    pub fn free_balance(lamports: u64, data_len: usize, reserved_unclaimed: u64) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(data_len);
        Ok(Self::free_balance_from_parts(
            lamports,
            min_rent,
            reserved_unclaimed,
        ))
    }

    /// Commission / validator identity split of a claimable amount.
    pub fn split_claim_amount(claimable: u64, commission_bps: u16) -> Result<(u64, u64)> {
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
        Ok((commission_amount, validator_amount))
    }

    /// Updates deficit, unpaid_streak, and status after a claim (or simulated shortfall).
    pub fn apply_claim_shortfall(
        deficit: &mut u64,
        unpaid_streak: &mut u8,
        status: &mut P2CSubscriptionStatus,
        grace_epochs: u8,
        shortfall: u64,
    ) -> Result<()> {
        if shortfall > 0 {
            *deficit = deficit.checked_add(shortfall).ok_or(ArithmeticError)?;
            *unpaid_streak = unpaid_streak.saturating_add(1);
            if *unpaid_streak > grace_epochs {
                *status = P2CSubscriptionStatus::Suspended;
            } else {
                *status = P2CSubscriptionStatus::InGrace;
            }
        } else {
            *unpaid_streak = 0;
            *status = P2CSubscriptionStatus::Active;
        }
        Ok(())
    }

    /// True if any ledger epoch is still unclaimed (blocks close).
    pub fn has_unclaimed_epochs(&self) -> bool {
        self.ledger.entries.iter().any(|e| !e.claimed)
    }

    /// After SOL is deposited and applied against open deficit (partial OK).
    /// Fully clearing deficit resets grace streak to Active.
    pub fn apply_deficit_cleared(
        deficit: &mut u64,
        unpaid_streak: &mut u8,
        status: &mut P2CSubscriptionStatus,
        paid: u64,
    ) -> Result<u64> {
        if paid == 0 {
            return Err(crate::ErrorCode::RewardsTooLow.into());
        }
        if *deficit == 0 {
            return Err(crate::ErrorCode::NoDeficit.into());
        }
        let applied = paid.min(*deficit);
        *deficit = deficit.saturating_sub(applied);
        if *deficit == 0 {
            *unpaid_streak = 0;
            *status = P2CSubscriptionStatus::Active;
        }
        Ok(applied)
    }

    /// Clear open deficit: balance already on PDA is split to commission + validator,
    /// then deficit reduced. Returns `(applied, commission, validator)`.
    pub fn clear_deficit(
        deficit: &mut u64,
        unpaid_streak: &mut u8,
        status: &mut P2CSubscriptionStatus,
        p2c_account: AccountInfo,
        commission_account: AccountInfo,
        validator_identity: AccountInfo,
        commission_bps: u16,
        amount: u64,
    ) -> Result<(u64, u64, u64)> {
        use crate::ErrorCode::*;

        if amount == 0 {
            return Err(RewardsTooLow.into());
        }
        if *deficit == 0 {
            return Err(NoDeficit.into());
        }

        let applied = amount.min(*deficit);
        let (commission_amount, validator_amount) =
            Self::split_claim_amount(applied, commission_bps)?;

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(p2c_account.data_len());
        let available = p2c_account.lamports().saturating_sub(min_rent);
        if available < applied {
            return Err(RewardsTooLow.into());
        }

        if commission_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                p2c_account.clone(),
                commission_account,
                commission_amount,
            )?;
        }
        if validator_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                p2c_account,
                validator_identity,
                validator_amount,
            )?;
        }

        Self::apply_deficit_cleared(deficit, unpaid_streak, status, applied)?;

        Ok((applied, commission_amount, validator_amount))
    }

    pub fn populate_on_init(
        &mut self,
        name: [u8; 32],
        validator_vote: Pubkey,
        initializer: Pubkey,
        manager_authority: Pubkey,
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        grace_epochs: u8,
        bump: u8,
    ) -> Result<()> {
        self.name = name;
        self.validator_vote = validator_vote;
        self.initializer = initializer;
        self.manager_authority = manager_authority;
        self.record_authority = record_authority;
        self.max_epoch_entries = max_epoch_entries;
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.grace_epochs = if grace_epochs == 0 {
            Self::DEFAULT_GRACE_EPOCHS
        } else {
            grace_epochs
        };
        self.block_reward_conversion_enabled = false;
        self.unpaid_streak = 0;
        self.status = P2CSubscriptionStatus::Active;
        self.deficit = 0;
        self.ledger = P2CSubscriptionLedger::default();
        self.bump = bump;
        self.validate()
    }

    pub fn validate_init_params(
        name: [u8; 32],
        manager_authority: Pubkey,
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        max_commission_bps: u16,
    ) -> Result<()> {
        if name == [0u8; 32] {
            return Err(InvalidRevenueName.into());
        }
        if manager_authority == Pubkey::default() || record_authority == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }
        if max_epoch_entries == 0 || max_epoch_entries as usize > MAX_REVENUE_EPOCH_ENTRIES_CAP {
            return Err(InvalidRevenueEpochCapacity.into());
        }
        validate_commission(commission_bps, commission_account, max_commission_bps)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        Self::validate_init_params(
            self.name,
            self.manager_authority,
            self.record_authority,
            self.max_epoch_entries,
            self.commission_bps,
            self.commission_account,
            MAX_COMMISSION_BPS,
        )?;
        if self.validator_vote == Pubkey::default() || self.initializer == Pubkey::default() {
            return Err(AccountValidationFailure.into());
        }
        Ok(())
    }

    pub fn auth_manager_signer(&self, signer: Pubkey) -> Result<()> {
        if signer != self.manager_authority {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Ok(())
    }

    pub fn auth_record_signer(&self, signer: Pubkey) -> Result<()> {
        if signer != self.record_authority {
            return Err(crate::ErrorCode::Unauthorized.into());
        }
        Ok(())
    }

    /// Record a new epoch charge once.
    pub fn record(&mut self, epoch: u64, stake: u64, amount_due: u64) -> Result<()> {
        if amount_due == 0 {
            return Err(crate::ErrorCode::RewardsTooLow.into());
        }
        let entry = P2CEpochEntry {
            epoch,
            stake,
            amount_due,
            amount_deducted: 0,
            claimed: false,
            block_reward_converted: !self.block_reward_conversion_enabled,
        };
        self.ledger.insert(entry, self.max_epoch_entries as usize)
    }

    /// How much more can be paid this call, given free prepaid.
    pub fn resolve_claimable(remaining_due: u64, free_balance: u64) -> (u64, u64) {
        let paid = remaining_due.min(free_balance);
        let still_short = remaining_due.saturating_sub(paid);
        (paid, still_short)
    }

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

    pub fn update_config(
        &mut self,
        commission_bps: u16,
        commission_account: Pubkey,
        block_reward_conversion_enabled: bool,
        grace_epochs: Option<u8>,
        record_authority: Option<Pubkey>,
    ) -> Result<()> {
        self.commission_bps = commission_bps;
        self.commission_account = commission_account;
        self.block_reward_conversion_enabled = block_reward_conversion_enabled;
        if let Some(g) = grace_epochs {
            self.grace_epochs = if g == 0 {
                Self::DEFAULT_GRACE_EPOCHS
            } else {
                g
            };
        }
        if let Some(ra) = record_authority {
            self.record_authority = ra;
        }
        self.validate()
    }

    /// Pay from free prepaid against this epoch.
    ///
    /// - Always notes and transfers `min(remaining_due, free)`.
    /// - Marks `claimed` when fully paid (`amount_deducted >= amount_due`).
    /// - Or, if `force_claim`, marks claimed with any leftover as `deficit` (+ grace).
    /// - Without `force_claim` and still underfunded: leaves epoch open (partial noted).
    ///
    /// Returns `(commission, validator, paid_this_ix, amount_deducted, shortfall, closed)`.
    pub fn claim_epoch(
        ledger: &mut P2CSubscriptionLedger,
        deficit: &mut u64,
        unpaid_streak: &mut u8,
        status: &mut P2CSubscriptionStatus,
        grace_epochs: u8,
        p2c_account: AccountInfo,
        commission_account: AccountInfo,
        validator_identity: AccountInfo,
        commission_bps: u16,
        epoch: u64,
        force_claim: bool,
    ) -> Result<(u64, u64, u64, u64, u64, bool)> {
        use crate::ErrorCode::*;

        let current_epoch = Clock::get()?.epoch;
        if current_epoch <= epoch {
            return Err(PrematureRevenueClaim.into());
        }

        let (amount_due, prior_deducted) = {
            let entry = ledger
                .entries
                .iter()
                .find(|e| e.epoch == epoch)
                .ok_or(EpochEntryNotFound)?;
            if entry.claimed {
                return Err(EpochAlreadyClaimed.into());
            }
            (entry.amount_due, entry.amount_deducted)
        };

        let remaining = amount_due.saturating_sub(prior_deducted);
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(p2c_account.data_len());
        // Prior partial payments already left the PDA; free is just spendable lamports.
        let free = Self::free_balance_from_parts(p2c_account.lamports(), min_rent, 0);
        let (paid, _) = Self::resolve_claimable(remaining, free);

        if paid == 0 && !force_claim && remaining > 0 {
            return Err(RewardsTooLow.into());
        }

        let (commission_amount, validator_amount) = Self::split_claim_amount(paid, commission_bps)?;

        if commission_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                p2c_account.clone(),
                commission_account.clone(),
                commission_amount,
            )?;
        }
        if validator_amount > 0 {
            RewardCollectionAccount::transfer_lamports(
                p2c_account.clone(),
                validator_identity,
                validator_amount,
            )?;
        }

        let amount_deducted = {
            let entry = ledger.get_mut(epoch)?;
            entry.amount_deducted = entry
                .amount_deducted
                .checked_add(paid)
                .ok_or(ArithmeticError)?;
            entry.amount_deducted
        };

        let remaining_after = amount_due.saturating_sub(amount_deducted);
        let close = remaining_after == 0 || force_claim;
        let shortfall = if close { remaining_after } else { 0 };

        if close {
            Self::apply_claim_shortfall(deficit, unpaid_streak, status, grace_epochs, shortfall)?;
            ledger.mark_claimed(epoch)?;
        }

        Ok((
            commission_amount,
            validator_amount,
            paid,
            amount_deducted,
            shortfall,
            close,
        ))
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
    fn revenue_share_v1_pda_differs_from_legacy() {
        let mut name = [0u8; 32];
        name[..7].copy_from_slice(b"rakurai");
        let vote = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let legacy = Pubkey::find_program_address(
            &RevenueShareAccount::pda_seeds(RevenueKind::Tip, &name, &vote),
            &program,
        )
        .0;
        let v1 = Pubkey::find_program_address(
            &RevenueShareAccountV1::pda_seeds_v1(RevenueKind::Tip, &name, &vote),
            &program,
        )
        .0;
        assert_ne!(legacy, v1);
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
    fn revenue_share_layout_sizes_differ() {
        assert_ne!(
            RevenueShareAccount::space_for(8),
            RevenueShareAccountV1::space_for(8)
        );
        assert!(RevenueShareAccountV1::space_for(8) > RevenueShareAccount::space_for(8));
    }

    #[test]
    fn deficit_update_apply_variants() {
        assert_eq!(DeficitUpdate::Set { value: 42 }.apply(7).unwrap(), 42);
        assert_eq!(DeficitUpdate::Clear.apply(100).unwrap(), 0);
        assert_eq!(DeficitUpdate::Increase { amount: 5 }.apply(10).unwrap(), 15);
        assert_eq!(DeficitUpdate::Decrease { amount: 3 }.apply(10).unwrap(), 7);
        assert_eq!(DeficitUpdate::Decrease { amount: 99 }.apply(10).unwrap(), 0);
    }

    #[test]
    fn credit_transferred_allows_over_settle() {
        let mut acc = test_revenue_share_account_v1(false);
        acc.record_revenue(10, 100).unwrap();
        acc.credit_transferred(10, 40).unwrap();
        acc.credit_transferred(10, 80).unwrap(); // 120 > recorded 100
        assert_eq!(acc.ledger.entries[0].transferred_amount, 120);
    }

    #[test]
    fn record_revenue_rakurai_tip_also_credits_transferred() {
        let mut acc = test_revenue_share_account_v1(false);
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
        let mut acc = test_revenue_share_account_v1(false);
        acc.record_revenue(10, 100).unwrap();
        assert_eq!(acc.ledger.entries[0].transferred_amount, 0);
    }

    #[test]
    fn legacy_record_revenue_only_updates_amount() {
        let mut acc = test_revenue_share_account(false);
        acc.record_revenue(10, 100).unwrap();
        assert_eq!(acc.ledger.entries[0].amount, 100);
        assert!(!acc.ledger.entries[0].claimed);
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
            bump: 0,
        }
    }

    fn test_revenue_share_account_v1(
        block_reward_conversion_enabled: bool,
    ) -> RevenueShareAccountV1 {
        let mut name = [0u8; 32];
        name[0] = b'X';
        RevenueShareAccountV1 {
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
            ledger: RevenueLedgerV1::default(),
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

    fn test_p2c_account(convert: bool) -> P2CSubscriptionAccount {
        P2CSubscriptionAccount {
            name: {
                let mut n = [0u8; 32];
                n[..3].copy_from_slice(b"p2c");
                n
            },
            validator_vote: Pubkey::new_unique(),
            initializer: Pubkey::new_unique(),
            manager_authority: Pubkey::new_unique(),
            record_authority: Pubkey::new_unique(),
            max_epoch_entries: 4,
            commission_bps: 2000,
            commission_account: Pubkey::new_unique(),
            grace_epochs: 2,
            block_reward_conversion_enabled: convert,
            unpaid_streak: 0,
            status: P2CSubscriptionStatus::Active,
            deficit: 0,
            ledger: P2CSubscriptionLedger::default(),
            bump: 0,
        }
    }

    #[test]
    fn p2c_resolve_claimable_partial_and_zero() {
        assert_eq!(P2CSubscriptionAccount::resolve_claimable(100, 40), (40, 60));
        assert_eq!(
            P2CSubscriptionAccount::resolve_claimable(100, 100),
            (100, 0)
        );
        assert_eq!(P2CSubscriptionAccount::resolve_claimable(50, 0), (0, 50));
        assert_eq!(P2CSubscriptionAccount::resolve_claimable(50, 200), (50, 0));
    }

    #[test]
    fn p2c_record_once_and_sets_due() {
        let mut acc = test_p2c_account(true);
        acc.record(10, 1_000_000, 100).unwrap();
        assert_eq!(acc.ledger.entries[0].amount_due, 100);
        assert_eq!(acc.ledger.entries[0].amount_deducted, 0);
        assert!(!acc.ledger.entries[0].claimed);
        assert!(!acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn p2c_record_rejects_duplicate_epoch() {
        let mut acc = test_p2c_account(false);
        acc.record(1, 1, 10).unwrap();
        assert!(acc.record(1, 1, 10).is_err());
    }

    #[test]
    fn p2c_ledger_reserved_unclaimed() {
        let mut acc = test_p2c_account(false);
        acc.record(1, 1, 100).unwrap();
        acc.record(2, 1, 50).unwrap();
        acc.ledger.entries[0].amount_deducted = 30;
        acc.ledger.entries[1].amount_deducted = 20;
        assert_eq!(acc.ledger.reserved_unclaimed().unwrap(), 50);
        acc.ledger.mark_claimed(1).unwrap();
        assert_eq!(acc.ledger.reserved_unclaimed().unwrap(), 20);
    }

    #[test]
    fn p2c_convert_requires_claimed() {
        let mut acc = test_p2c_account(true);
        acc.record(7, 1, 10).unwrap();
        assert!(acc.mark_epoch_converted_to_block_reward(7).is_err());
        acc.ledger.mark_claimed(7).unwrap();
        acc.mark_epoch_converted_to_block_reward(7).unwrap();
        assert!(acc.ledger.entries[0].block_reward_converted);
    }

    #[test]
    fn p2c_pda_seeds_include_name_and_vote() {
        let mut name = [0u8; 32];
        name[..3].copy_from_slice(b"abc");
        let vote = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let (a, _) = Pubkey::find_program_address(
            &P2CSubscriptionAccount::pda_seeds(&name, &vote),
            &program,
        );
        let mut name2 = name;
        name2[0] = b'z';
        let (b, _) = Pubkey::find_program_address(
            &P2CSubscriptionAccount::pda_seeds(&name2, &vote),
            &program,
        );
        assert_ne!(a, b);
    }

    #[test]
    fn p2c_free_balance_excludes_rent_and_reserved() {
        assert_eq!(
            P2CSubscriptionAccount::free_balance_from_parts(1_000, 200, 300),
            500
        );
        assert_eq!(
            P2CSubscriptionAccount::free_balance_from_parts(100, 200, 0),
            0
        );
    }

    #[test]
    fn p2c_split_claim_twenty_percent() {
        let (c, v) = P2CSubscriptionAccount::split_claim_amount(100, 2000).unwrap();
        assert_eq!(c, 20);
        assert_eq!(v, 80);
        let (c0, v0) = P2CSubscriptionAccount::split_claim_amount(100, 0).unwrap();
        assert_eq!(c0, 0);
        assert_eq!(v0, 100);
    }

    #[test]
    fn p2c_grace_streak_active_ingrace_suspended() {
        let mut deficit = 0u64;
        let mut streak = 0u8;
        let mut status = P2CSubscriptionStatus::Active;
        let grace = 2u8;

        P2CSubscriptionAccount::apply_claim_shortfall(
            &mut deficit,
            &mut streak,
            &mut status,
            grace,
            0,
        )
        .unwrap();
        assert_eq!(streak, 0);
        assert_eq!(status, P2CSubscriptionStatus::Active);

        P2CSubscriptionAccount::apply_claim_shortfall(
            &mut deficit,
            &mut streak,
            &mut status,
            grace,
            10,
        )
        .unwrap();
        assert_eq!(deficit, 10);
        assert_eq!(streak, 1);
        assert_eq!(status, P2CSubscriptionStatus::InGrace);

        P2CSubscriptionAccount::apply_claim_shortfall(
            &mut deficit,
            &mut streak,
            &mut status,
            grace,
            5,
        )
        .unwrap();
        assert_eq!(deficit, 15);
        assert_eq!(streak, 2);
        assert_eq!(status, P2CSubscriptionStatus::InGrace);

        P2CSubscriptionAccount::apply_claim_shortfall(
            &mut deficit,
            &mut streak,
            &mut status,
            grace,
            1,
        )
        .unwrap();
        assert_eq!(streak, 3);
        assert_eq!(status, P2CSubscriptionStatus::Suspended);

        // full pay resets
        P2CSubscriptionAccount::apply_claim_shortfall(
            &mut deficit,
            &mut streak,
            &mut status,
            grace,
            0,
        )
        .unwrap();
        assert_eq!(streak, 0);
        assert_eq!(status, P2CSubscriptionStatus::Active);
        assert_eq!(deficit, 16); // 10 + 5 + 1; full pay does not write off deficit
    }

    #[test]
    fn p2c_clear_deficit_partial_and_full_resets_grace() {
        let mut deficit = 100u64;
        let mut streak = 3u8;
        let mut status = P2CSubscriptionStatus::Suspended;

        let applied = P2CSubscriptionAccount::apply_deficit_cleared(
            &mut deficit,
            &mut streak,
            &mut status,
            40,
        )
        .unwrap();
        assert_eq!(applied, 40);
        assert_eq!(deficit, 60);
        assert_eq!(status, P2CSubscriptionStatus::Suspended);

        let applied2 = P2CSubscriptionAccount::apply_deficit_cleared(
            &mut deficit,
            &mut streak,
            &mut status,
            1000,
        )
        .unwrap();
        assert_eq!(applied2, 60);
        assert_eq!(deficit, 0);
        assert_eq!(streak, 0);
        assert_eq!(status, P2CSubscriptionStatus::Active);
    }

    #[test]
    fn p2c_clear_deficit_rejects_zero_open() {
        let mut deficit = 0u64;
        let mut streak = 0u8;
        let mut status = P2CSubscriptionStatus::Active;
        assert!(P2CSubscriptionAccount::apply_deficit_cleared(
            &mut deficit,
            &mut streak,
            &mut status,
            10,
        )
        .is_err());
    }

    #[test]
    fn p2c_has_unclaimed_blocks_close_signal() {
        let mut acc = test_p2c_account(false);
        assert!(!acc.has_unclaimed_epochs());
        acc.record(1, 10, 50).unwrap();
        assert!(acc.has_unclaimed_epochs());
        acc.ledger.mark_claimed(1).unwrap();
        assert!(!acc.has_unclaimed_epochs());
    }

    #[test]
    fn p2c_init_params_require_manager_and_name() {
        assert!(P2CSubscriptionAccount::validate_init_params(
            [0u8; 32],
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            4,
            2000,
            Pubkey::new_unique(),
            10_000,
        )
        .is_err());
        assert!(P2CSubscriptionAccount::validate_init_params(
            {
                let mut n = [0u8; 32];
                n[0] = b'x';
                n
            },
            Pubkey::default(),
            Pubkey::new_unique(),
            4,
            2000,
            Pubkey::new_unique(),
            10_000,
        )
        .is_err());
        assert!(P2CSubscriptionAccount::validate_init_params(
            {
                let mut n = [0u8; 32];
                n[0] = b'x';
                n
            },
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            4,
            2000,
            Pubkey::new_unique(),
            10_000,
        )
        .is_ok());
    }

    #[test]
    fn p2c_default_grace_epochs_is_two() {
        assert_eq!(P2CSubscriptionAccount::DEFAULT_GRACE_EPOCHS, 2);
        let mut acc = test_p2c_account(false);
        acc.populate_on_init(
            acc.name,
            acc.validator_vote,
            acc.initializer,
            acc.manager_authority,
            acc.record_authority,
            4,
            2000,
            acc.commission_account,
            0, // zero → default 2
            0,
        )
        .unwrap();
        assert_eq!(acc.grace_epochs, 2);
        assert_eq!(acc.status, P2CSubscriptionStatus::Active);
    }
}
