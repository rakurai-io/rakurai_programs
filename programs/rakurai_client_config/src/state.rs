use anchor_lang::prelude::*;

pub const GLOBAL_CONFIG_SEED: &[u8] = b"global-validator-config";
pub const VALIDATOR_CONFIG_SEED: &[u8] = b"validator-config";
pub const VALIDATOR_PROPOSAL_SEED: &[u8] = b"validator-proposal";
pub const CONFIG_STAGING_SEED: &[u8] = b"config-staging";
pub const STAGING_TAG_GLOBAL: &[u8] = b"g";
pub const STAGING_TAG_VALIDATOR: &[u8] = b"v";
pub const STAGING_TAG_PROPOSAL: &[u8] = b"p";
pub const STAGING_KIND_GLOBAL: u8 = 0;
pub const STAGING_KIND_VALIDATOR: u8 = 1;
pub const STAGING_KIND_PROPOSAL: u8 = 2;
/// Max Borsh payload accepted in a staging PDA (keeps realloc / CU bounded).
pub const MAX_STAGING_BYTES: u32 = 100_000;
pub const NAME_LEN: usize = 32;

/// Absolute safety caps for account `ConfigLimits` (cannot be raised further).
/// Prevents CU / realloc blowups if limits are mis-set.
pub const ABSOLUTE_MAX_URL_LEN: u16 = 1024;
pub const ABSOLUTE_MAX_SETS_PER_SECTION: u8 = 64;
pub const ABSOLUTE_MAX_URLS_PER_SET: u8 = 32;
pub const ABSOLUTE_MAX_VP_ENTRIES_PER_SET: u8 = 255;

/// Size-cap fields for one limits schema version.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigLimitsV1 {
    pub max_url_len: u16,
    pub max_sets_per_section: u8,
    pub max_urls_per_set: u8,
    pub max_vp_entries_per_set: u8,
}

impl Default for ConfigLimitsV1 {
    fn default() -> Self {
        Self {
            max_url_len: 256,
            max_sets_per_section: 16,
            max_urls_per_set: 8,
            max_vp_entries_per_set: 64,
        }
    }
}

impl ConfigLimitsV1 {
    pub fn validate(&self) -> Result<()> {
        require!(
            self.max_url_len > 0
                && self.max_sets_per_section > 0
                && self.max_urls_per_set > 0
                && self.max_vp_entries_per_set > 0,
            crate::ConfigError::InvalidLimits
        );
        require!(
            self.max_url_len <= ABSOLUTE_MAX_URL_LEN
                && self.max_sets_per_section <= ABSOLUTE_MAX_SETS_PER_SECTION
                && self.max_urls_per_set <= ABSOLUTE_MAX_URLS_PER_SET
                && self.max_vp_entries_per_set <= ABSOLUTE_MAX_VP_ENTRIES_PER_SET,
            crate::ConfigError::InvalidLimits
        );
        Ok(())
    }
}

/// Versioned size caps stored on Global / Validator / Proposal.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigLimits {
    V1(ConfigLimitsV1),
}

impl Default for ConfigLimits {
    fn default() -> Self {
        Self::V1(ConfigLimitsV1::default())
    }
}

impl ConfigLimits {
    pub fn as_v1(&self) -> Result<&ConfigLimitsV1> {
        match self {
            Self::V1(v) => Ok(v),
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.as_v1()?.validate()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Uuid(pub [u8; NAME_LEN]);

impl Uuid {
    pub fn from_str_truncated(s: &str) -> Self {
        let mut bytes = [0u8; NAME_LEN];
        let src = s.as_bytes();
        let len = src.len().min(NAME_LEN);
        bytes[..len].copy_from_slice(&src[..len]);
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; NAME_LEN] {
        &self.0
    }
}

/// Versioned config payload shared by global and validator PDAs.
///
/// Discriminant 0 (`V1`) is reserved so existing `V2` accounts (discriminant 1)
/// keep decoding after ConfigV1 was removed. `V3` is discriminant 2.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum Config {
    /// Former ConfigV1 — unsupported. Writing or validating this variant fails.
    V1,
    V2(ConfigV2),
    V3(ConfigV3),
}

impl Config {
    pub fn validate(&self, limits: &ConfigLimits) -> Result<()> {
        match self {
            Config::V1 => Err(error!(crate::ConfigError::UnexpectedConfigVersion)),
            Config::V2(v2) => v2.validate(limits),
            Config::V3(v3) => v3.validate(limits),
        }
    }

    pub fn as_v2(&self) -> Result<&ConfigV2> {
        match self {
            Config::V2(v2) => Ok(v2),
            Config::V1 | Config::V3(_) => {
                Err(error!(crate::ConfigError::UnexpectedConfigVersion))
            }
        }
    }

    pub fn as_v3(&self) -> Result<&ConfigV3> {
        match self {
            Config::V3(v3) => Ok(v3),
            Config::V1 | Config::V2(_) => {
                Err(error!(crate::ConfigError::UnexpectedConfigVersion))
            }
        }
    }

    pub fn to_v2(&self) -> Result<ConfigV2> {
        Ok(self.as_v2()?.clone())
    }

    /// Prefer V3; project V2 by copying the global TPU flag onto every P2C URL.
    pub fn to_v3(&self) -> Result<ConfigV3> {
        match self {
            Config::V3(v3) => Ok(v3.clone()),
            Config::V2(v2) => Ok(ConfigV3::from_v2(v2.clone())),
            Config::V1 => Err(error!(crate::ConfigError::UnexpectedConfigVersion)),
        }
    }

    /// Rewrite a V2 payload as V3 in place (global flag → per-URL).
    /// Returns `true` if the account data version changed.
    pub fn migrate_to_v3(&mut self) -> bool {
        let v2 = match self {
            Config::V2(v2) => v2.clone(),
            Config::V1 | Config::V3(_) => return false,
        };
        *self = Config::V3(ConfigV3::from_v2(v2));
        true
    }
}

/// V2 payload: section lists plus global `enable_tpu_p2c_update`.
///
/// `String` / `Vec` fields are dynamically sized: account bytes grow/shrink via
/// `realloc_to_fit` on every init/update. Caps come from account `ConfigLimits`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ConfigV2 {
    pub block_engine: BlockEngineV1,
    pub p2c: P2cV1,
    pub virtual_priority: VirtualPriorityV1,
    pub enable_tpu_p2c_update: bool,
}

impl ConfigV2 {
    pub fn empty() -> Self {
        Self {
            block_engine: BlockEngineV1 { sets: vec![] },
            p2c: P2cV1 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
            enable_tpu_p2c_update: false,
        }
    }

    pub fn validate(&self, limits: &ConfigLimits) -> Result<()> {
        validate_sections(
            &self.block_engine,
            &self.p2c,
            &self.virtual_priority,
            limits,
        )
    }
}

/// V3 payload: same sections as V2, but `enable_tpu_p2c_update` lives per P2C URL.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ConfigV3 {
    pub block_engine: BlockEngineV1,
    pub p2c: P2cV3,
    pub virtual_priority: VirtualPriorityV1,
}

impl ConfigV3 {
    pub fn empty() -> Self {
        Self {
            block_engine: BlockEngineV1 { sets: vec![] },
            p2c: P2cV3 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
        }
    }

    pub fn from_v2(v2: ConfigV2) -> Self {
        let enable = v2.enable_tpu_p2c_update;
        Self {
            block_engine: v2.block_engine,
            p2c: P2cV3 {
                sets: v2
                    .p2c
                    .sets
                    .into_iter()
                    .map(|entry| P2cEntryV3 {
                        name: entry.name,
                        url: entry
                            .url
                            .into_iter()
                            .map(|u| P2cConfig {
                                url: u.url,
                                enable_tpu_p2c_update: enable,
                            })
                            .collect(),
                    })
                    .collect(),
            },
            virtual_priority: v2.virtual_priority,
        }
    }

    pub fn validate(&self, limits: &ConfigLimits) -> Result<()> {
        validate_sections_v3(
            &self.block_engine,
            &self.p2c,
            &self.virtual_priority,
            limits,
        )
    }

    /// True if any P2C endpoint enables TPU→P2C forwarding.
    pub fn any_tpu_p2c_update_enabled(&self) -> bool {
        self.p2c
            .sets
            .iter()
            .flat_map(|set| set.url.iter())
            .any(|u| u.enable_tpu_p2c_update)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct BlockEngineV1 {
    /// Dynamic list of named endpoint groups; realloc when this grows/shrinks.
    pub sets: Vec<BlockEngineEntryV1>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct BlockEngineEntryV1 {
    pub name: Uuid,
    pub url: Vec<BlockEngineConfig>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct BlockEngineConfig {
    pub url: String,
    /// Bundles admitted per `period_ms`. 0 = unlimited.
    pub max_bundles: u32,
    /// Quota window in milliseconds. 0 = unlimited.
    pub period_ms: u32,
    /// Token-bucket capacity. If 0 and quota is set, treat as equal to `max_bundles`.
    pub max_bundle_burst: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cV1 {
    /// Dynamic list of named endpoint groups; realloc when this grows/shrinks.
    pub sets: Vec<P2cEntryV1>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cEntryV1 {
    pub name: Uuid,
    pub url: Vec<P2cUrl>,
}

/// ConfigV2 P2C endpoint (url only). Same Borsh layout as the former `P2cConfig`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cUrl {
    pub url: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cV3 {
    pub sets: Vec<P2cEntryV3>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cEntryV3 {
    pub name: Uuid,
    pub url: Vec<P2cConfig>,
}

/// ConfigV3 P2C endpoint with per-URL TPU→P2C gate.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cConfig {
    pub url: String,
    pub enable_tpu_p2c_update: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct VirtualPriorityV1 {
    /// Dynamic list of named priority groups; realloc when this grows/shrinks.
    pub sets: Vec<VirtualPriorityEntryV1>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct VirtualPriorityEntryV1 {
    pub name: Uuid,
    pub url: Vec<VirtualPriorityConfig>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct VirtualPriorityConfig {
    pub key: Pubkey,
    /// Fraction of tip used for virtual priority; must be in `[0.0, 1.0]`.
    pub value: f64,
}

/// Effective payload for a vote: validator PDA if present, otherwise global.
/// No merge — the chosen account is used as-is.
pub fn effective_config<'a>(global: &'a Config, validator: Option<&'a Config>) -> &'a Config {
    validator.unwrap_or(global)
}

#[account]
pub struct GlobalConfig {
    pub manager: Pubkey,
    pub bump: u8,
    pub limits: ConfigLimits,
    pub config: Config,
}

#[account]
pub struct ValidatorConfig {
    pub manager: Pubkey,
    /// Key allowed to create/update the proposal PDA for this vote.
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub bump: u8,
    pub limits: ConfigLimits,
    pub config: Config,
}

/// Per-vote draft config. Live clients ignore this until manager approve.
#[account]
pub struct ValidatorProposal {
    pub vote: Pubkey,
    /// Operator who proposed; receives rent on approve/reject close.
    pub operator: Pubkey,
    pub bump: u8,
    pub limits: ConfigLimits,
    pub config: Config,
}

/// Ephemeral upload buffer. Create → append chunks → commit into live PDA → close.
#[account]
pub struct ConfigStaging {
    pub authority: Pubkey,
    pub bump: u8,
    /// 0 = global, 1 = validator, 2 = proposal.
    pub kind: u8,
    /// Vote pubkey for validator/proposal staging; default for global.
    pub vote: Pubkey,
    pub expected_len: u32,
    pub data: Vec<u8>,
}

impl GlobalConfig {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(manager: Pubkey, limits: ConfigLimits, config: &Config) -> Result<usize> {
        Self::serialized_len(&Self {
            manager,
            bump: 0,
            limits,
            config: config.clone(),
        })
    }

    pub fn realloc_to_fit<'info>(
        account: &Account<'info, Self>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
    ) -> Result<()> {
        let new_len = Self::serialized_len(account)?;
        realloc_account_to_fit(&account.to_account_info(), payer, system_program, new_len)
    }
}

impl ValidatorConfig {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(
        manager: Pubkey,
        operator: Pubkey,
        vote: Pubkey,
        limits: ConfigLimits,
        config: &Config,
    ) -> Result<usize> {
        Self::serialized_len(&Self {
            manager,
            operator,
            vote,
            bump: 0,
            limits,
            config: config.clone(),
        })
    }

    pub fn realloc_to_fit<'info>(
        account: &Account<'info, Self>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
    ) -> Result<()> {
        let new_len = Self::serialized_len(account)?;
        realloc_account_to_fit(&account.to_account_info(), payer, system_program, new_len)
    }
}

impl ConfigStaging {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(
        authority: Pubkey,
        kind: u8,
        vote: Pubkey,
        expected_len: u32,
    ) -> Result<usize> {
        Self::serialized_len(&Self {
            authority,
            bump: 0,
            kind,
            vote,
            expected_len,
            data: Vec::new(),
        })
    }

    pub fn realloc_to_fit<'info>(
        account: &Account<'info, Self>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
    ) -> Result<()> {
        let new_len = Self::serialized_len(account)?;
        realloc_account_to_fit(&account.to_account_info(), payer, system_program, new_len)
    }

    pub fn append(&mut self, chunk: &[u8]) -> Result<()> {
        let new_len = (self.data.len() as u32)
            .checked_add(chunk.len() as u32)
            .ok_or(crate::ConfigError::StagingTooLarge)?;
        require!(
            new_len <= self.expected_len,
            crate::ConfigError::StagingLengthMismatch
        );
        self.data.extend_from_slice(chunk);
        Ok(())
    }

    pub fn parse_config(&self) -> Result<Config> {
        require!(
            self.data.len() as u32 == self.expected_len,
            crate::ConfigError::StagingIncomplete
        );
        Config::try_from_slice(&self.data)
            .map_err(|_| error!(crate::ConfigError::StagingDeserializeFailed))
    }
}

impl ValidatorProposal {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(
        vote: Pubkey,
        operator: Pubkey,
        limits: ConfigLimits,
        config: &Config,
    ) -> Result<usize> {
        Self::serialized_len(&Self {
            vote,
            operator,
            bump: 0,
            limits,
            config: config.clone(),
        })
    }

    pub fn realloc_to_fit<'info>(
        account: &Account<'info, Self>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
    ) -> Result<()> {
        let new_len = Self::serialized_len(account)?;
        realloc_account_to_fit(&account.to_account_info(), payer, system_program, new_len)
    }
}

fn validate_sections(
    block_engine: &BlockEngineV1,
    p2c: &P2cV1,
    virtual_priority: &VirtualPriorityV1,
    limits: &ConfigLimits,
) -> Result<()> {
    let limits = limits.as_v1()?;
    require!(
        block_engine.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );
    require!(
        p2c.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );
    require!(
        virtual_priority.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );

    for entry in &block_engine.sets {
        require!(
            entry.url.len() <= limits.max_urls_per_set as usize,
            crate::ConfigError::TooManyUrls
        );
        validate_urls(
            entry.url.iter().map(|c| c.url.as_str()),
            limits.max_url_len as usize,
        )?;
    }
    for entry in &p2c.sets {
        require!(
            entry.url.len() <= limits.max_urls_per_set as usize,
            crate::ConfigError::TooManyUrls
        );
        validate_urls(
            entry.url.iter().map(|c| c.url.as_str()),
            limits.max_url_len as usize,
        )?;
    }
    for entry in &virtual_priority.sets {
        require!(
            entry.url.len() <= limits.max_vp_entries_per_set as usize,
            crate::ConfigError::TooManyVpEntries
        );
        for vp in &entry.url {
            require!(
                vp.value.is_finite() && (0.0..=1.0).contains(&vp.value),
                crate::ConfigError::InvalidVpValue
            );
        }
    }
    Ok(())
}

fn validate_sections_v3(
    block_engine: &BlockEngineV1,
    p2c: &P2cV3,
    virtual_priority: &VirtualPriorityV1,
    limits: &ConfigLimits,
) -> Result<()> {
    let limits = limits.as_v1()?;
    require!(
        block_engine.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );
    require!(
        p2c.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );
    require!(
        virtual_priority.sets.len() <= limits.max_sets_per_section as usize,
        crate::ConfigError::TooManySets
    );

    for entry in &block_engine.sets {
        require!(
            entry.url.len() <= limits.max_urls_per_set as usize,
            crate::ConfigError::TooManyUrls
        );
        validate_urls(
            entry.url.iter().map(|c| c.url.as_str()),
            limits.max_url_len as usize,
        )?;
    }
    for entry in &p2c.sets {
        require!(
            entry.url.len() <= limits.max_urls_per_set as usize,
            crate::ConfigError::TooManyUrls
        );
        validate_urls(
            entry.url.iter().map(|c| c.url.as_str()),
            limits.max_url_len as usize,
        )?;
    }
    for entry in &virtual_priority.sets {
        require!(
            entry.url.len() <= limits.max_vp_entries_per_set as usize,
            crate::ConfigError::TooManyVpEntries
        );
        for vp in &entry.url {
            require!(
                vp.value.is_finite() && (0.0..=1.0).contains(&vp.value),
                crate::ConfigError::InvalidVpValue
            );
        }
    }
    Ok(())
}

fn validate_urls<'a>(urls: impl Iterator<Item = &'a str>, max_url_len: usize) -> Result<()> {
    for url in urls {
        require!(!url.is_empty(), crate::ConfigError::UrlEmpty);
        require!(url.len() <= max_url_len, crate::ConfigError::UrlTooLong);
    }
    Ok(())
}

pub fn realloc_account_to_fit<'info>(
    account: &AccountInfo<'info>,
    payer: &Signer<'info>,
    system_program: &Program<'info, System>,
    new_len: usize,
) -> Result<()> {
    let old_len = account.data_len();
    if new_len == old_len {
        return Ok(());
    }

    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(new_len);
    let lamports = account.lamports();

    if new_len > old_len {
        let required = new_minimum_balance.saturating_sub(lamports);
        if required > 0 {
            let cpi_accounts = anchor_lang::system_program::Transfer {
                from: payer.to_account_info(),
                to: account.clone(),
            };
            let cpi_ctx = CpiContext::new(system_program.to_account_info(), cpi_accounts);
            anchor_lang::system_program::transfer(cpi_ctx, required)?;
        }
        account.realloc(new_len, false)?;
    } else {
        account.realloc(new_len, false)?;
        let excess = lamports.saturating_sub(new_minimum_balance);
        if excess > 0 {
            **account.try_borrow_mut_lamports()? = account
                .lamports()
                .checked_sub(excess)
                .ok_or(ProgramError::InsufficientFunds)?;
            **payer.try_borrow_mut_lamports()? = payer
                .lamports()
                .checked_add(excess)
                .ok_or(ProgramError::InsufficientFunds)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be_entry(name: &str, url: &str) -> BlockEngineEntryV1 {
        BlockEngineEntryV1 {
            name: Uuid::from_str_truncated(name),
            url: vec![BlockEngineConfig {
                url: url.to_string(),
                max_bundles: 0,
                period_ms: 0,
                max_bundle_burst: 0,
            }],
        }
    }

    fn config_with_be(entries: Vec<BlockEngineEntryV1>) -> Config {
        config_v2_with_be(entries, false)
    }

    fn config_v2_with_be(entries: Vec<BlockEngineEntryV1>, enable_tpu_p2c_update: bool) -> Config {
        Config::V2(ConfigV2 {
            block_engine: BlockEngineV1 { sets: entries },
            p2c: P2cV1 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
            enable_tpu_p2c_update,
        })
    }

    #[test]
    fn effective_without_validator_returns_global() {
        let global = config_with_be(vec![be_entry("a", "https://global.example")]);
        let chosen = effective_config(&global, None);
        assert_eq!(chosen, &global);
    }

    #[test]
    fn effective_with_validator_returns_validator_only() {
        let global = config_with_be(vec![
            be_entry("a", "https://global.example"),
            be_entry("b", "https://global-b.example"),
        ]);
        let validator = config_with_be(vec![be_entry("a", "https://validator.example")]);
        let chosen = effective_config(&global, Some(&validator));
        assert_eq!(chosen, &validator);
        assert_eq!(chosen.to_v2().unwrap().block_engine.sets.len(), 1);
    }

    #[test]
    fn v2_keeps_discriminant_one_after_v1_removal() {
        let v2 = config_v2_with_be(vec![be_entry("a", "https://a")], true);
        let bytes = v2.try_to_vec().unwrap();
        assert_eq!(bytes[0], 1);
        let decoded = Config::try_from_slice(&bytes).unwrap();
        assert!(matches!(decoded, Config::V2(_)));
    }

    #[test]
    fn reserved_v1_variant_rejects_validate() {
        let limits = ConfigLimits::default();
        assert!(Config::V1.validate(&limits).is_err());
    }

    #[test]
    fn validate_rejects_url_too_long() {
        let limits = ConfigLimits::default();
        let long = "x".repeat(limits.as_v1().unwrap().max_url_len as usize + 1);
        let cfg = config_with_be(vec![be_entry("a", &long)]);
        assert!(cfg.validate(&limits).is_err());
    }

    #[test]
    fn validate_rejects_too_many_sets() {
        let limits = ConfigLimits::V1(ConfigLimitsV1 {
            max_sets_per_section: 1,
            ..ConfigLimitsV1::default()
        });
        let cfg = config_with_be(vec![be_entry("a", "https://a"), be_entry("b", "https://b")]);
        assert!(cfg.validate(&limits).is_err());
    }

    #[test]
    fn limits_reject_above_absolute() {
        let bad = ConfigLimits::V1(ConfigLimitsV1 {
            max_url_len: ABSOLUTE_MAX_URL_LEN + 1,
            ..ConfigLimitsV1::default()
        });
        assert!(bad.validate().is_err());
    }

    #[test]
    fn limits_reject_zero() {
        let bad = ConfigLimits::V1(ConfigLimitsV1 {
            max_sets_per_section: 0,
            ..ConfigLimitsV1::default()
        });
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_vp_value_outside_unit_interval() {
        let limits = ConfigLimits::default();
        let cfg = Config::V2(ConfigV2 {
            block_engine: BlockEngineV1 { sets: vec![] },
            p2c: P2cV1 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 {
                sets: vec![VirtualPriorityEntryV1 {
                    name: Uuid::from_str_truncated("vp"),
                    url: vec![VirtualPriorityConfig {
                        key: Pubkey::default(),
                        value: 1.1,
                    }],
                }],
            },
            enable_tpu_p2c_update: false,
        });
        assert!(cfg.validate(&limits).is_err());
    }

    #[test]
    fn staging_parse_round_trip() {
        let cfg = config_with_be(vec![be_entry("a", "https://a.example")]);
        let bytes = cfg.try_to_vec().unwrap();
        let staging = ConfigStaging {
            authority: Pubkey::default(),
            bump: 255,
            kind: STAGING_KIND_GLOBAL,
            vote: Pubkey::default(),
            expected_len: bytes.len() as u32,
            data: bytes,
        };
        assert_eq!(staging.parse_config().unwrap(), cfg);
    }

    #[test]
    fn staging_parse_round_trip_v2() {
        let cfg = config_v2_with_be(vec![be_entry("a", "https://a.example")], true);
        let bytes = cfg.try_to_vec().unwrap();
        let staging = ConfigStaging {
            authority: Pubkey::default(),
            bump: 255,
            kind: STAGING_KIND_GLOBAL,
            vote: Pubkey::default(),
            expected_len: bytes.len() as u32,
            data: bytes,
        };
        assert_eq!(staging.parse_config().unwrap(), cfg);
    }

    #[test]
    fn migrate_v2_to_v3_copies_global_flag_onto_each_url() {
        let mut cfg = Config::V2(ConfigV2 {
            block_engine: BlockEngineV1 { sets: vec![] },
            p2c: P2cV1 {
                sets: vec![P2cEntryV1 {
                    name: Uuid::from_str_truncated("p2c"),
                    url: vec![
                        P2cUrl {
                            url: "https://a".to_string(),
                        },
                        P2cUrl {
                            url: "https://b".to_string(),
                        },
                    ],
                }],
            },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
            enable_tpu_p2c_update: true,
        });
        assert!(cfg.migrate_to_v3());
        let v3 = cfg.to_v3().unwrap();
        assert!(v3.any_tpu_p2c_update_enabled());
        assert_eq!(v3.p2c.sets[0].url.len(), 2);
        assert!(v3.p2c.sets[0].url.iter().all(|u| u.enable_tpu_p2c_update));
        assert!(!cfg.migrate_to_v3());
    }

    #[test]
    fn v3_discriminant_is_two() {
        let cfg = Config::V3(ConfigV3::empty());
        let bytes = cfg.try_to_vec().unwrap();
        assert_eq!(bytes[0], 2);
    }
}
