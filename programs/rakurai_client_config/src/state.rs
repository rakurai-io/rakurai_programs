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
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum Config {
    V1(ConfigV1),
    V2(ConfigV2),
}

impl Config {
    pub fn validate(&self, limits: &ConfigLimits) -> Result<()> {
        match self {
            Config::V1(v1) => v1.validate(limits),
            Config::V2(v2) => v2.validate(limits),
        }
    }

    pub fn as_v1(&self) -> Result<&ConfigV1> {
        match self {
            Config::V1(v1) => Ok(v1),
            Config::V2(_) => Err(error!(crate::ConfigError::UnexpectedConfigVersion)),
        }
    }

    pub fn into_v1(self) -> Result<ConfigV1> {
        match self {
            Config::V1(v1) => Ok(v1),
            Config::V2(_) => Err(error!(crate::ConfigError::UnexpectedConfigVersion)),
        }
    }

    pub fn as_v2(&self) -> Result<&ConfigV2> {
        match self {
            Config::V2(v2) => Ok(v2),
            Config::V1(_) => Err(error!(crate::ConfigError::UnexpectedConfigVersion)),
        }
    }

    /// V1 payloads become V2 with `enable_tpu_p2c_update = false`.
    pub fn to_v2(&self) -> ConfigV2 {
        match self {
            Config::V1(v1) => ConfigV2::from_v1(v1.clone()),
            Config::V2(v2) => v2.clone(),
        }
    }

    /// Rewrite a V1 payload as V2 in place (`enable_tpu_p2c_update = false`).
    /// Returns `true` if the account data version changed.
    pub fn migrate_to_v2(&mut self) -> bool {
        let v1 = match self {
            Config::V1(v1) => v1.clone(),
            Config::V2(_) => return false,
        };
        *self = Config::V2(ConfigV2::from_v1(v1));
        true
    }
}

/// Shared V1 payload for global and validator PDAs (no mode).
///
/// `String` / `Vec` fields are dynamically sized: account bytes grow/shrink via
/// `realloc_to_fit` on every init/update. Caps come from account `ConfigLimits`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ConfigV1 {
    pub block_engine: BlockEngineV1,
    pub p2c: P2cV1,
    pub virtual_priority: VirtualPriorityV1,
}

impl ConfigV1 {
    pub fn empty() -> Self {
        Self {
            block_engine: BlockEngineV1 { sets: vec![] },
            p2c: P2cV1 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
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

/// V2 payload: V1 sections plus `enable_tpu_p2c_update`.
/// Existing V1 accounts migrate with the flag defaulting to `false`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ConfigV2 {
    pub block_engine: BlockEngineV1,
    pub p2c: P2cV1,
    pub virtual_priority: VirtualPriorityV1,
    pub enable_tpu_p2c_update: bool,
}

impl ConfigV2 {
    pub fn empty() -> Self {
        ConfigV1::empty().into()
    }

    pub fn from_v1(v1: ConfigV1) -> Self {
        v1.into()
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

impl From<ConfigV1> for ConfigV2 {
    fn from(v1: ConfigV1) -> Self {
        Self {
            block_engine: v1.block_engine,
            p2c: v1.p2c,
            virtual_priority: v1.virtual_priority,
            enable_tpu_p2c_update: false,
        }
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
    pub url: Vec<P2cConfig>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct P2cConfig {
    pub url: String,
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
        Config::V1(ConfigV1 {
            block_engine: BlockEngineV1 { sets: entries },
            p2c: P2cV1 { sets: vec![] },
            virtual_priority: VirtualPriorityV1 { sets: vec![] },
        })
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
        assert_eq!(chosen.to_v2().block_engine.sets.len(), 1);
    }

    #[test]
    fn v1_borsh_still_deserializes_after_v2_variant() {
        let v1 = config_with_be(vec![be_entry("a", "https://a")]);
        let bytes = v1.try_to_vec().unwrap();
        let decoded = Config::try_from_slice(&bytes).unwrap();
        assert!(matches!(decoded, Config::V1(_)));
    }

    #[test]
    fn migrate_v1_to_v2_defaults_flag_false() {
        let mut cfg = config_with_be(vec![be_entry("a", "https://a")]);
        assert!(cfg.migrate_to_v2());
        let v2 = cfg.to_v2();
        assert!(!v2.enable_tpu_p2c_update);
        assert_eq!(v2.block_engine.sets.len(), 1);
        assert_eq!(v2.block_engine.sets[0].url[0].url, "https://a");
        assert!(!cfg.migrate_to_v2());
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
        let cfg = Config::V1(ConfigV1 {
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
}
