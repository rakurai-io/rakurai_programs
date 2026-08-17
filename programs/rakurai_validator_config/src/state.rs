use anchor_lang::prelude::*;

pub const GLOBAL_CONFIG_SEED: &[u8] = b"global-validator-config";
pub const VALIDATOR_CONFIG_SEED: &[u8] = b"validator-config";
pub const VALIDATOR_PROPOSAL_SEED: &[u8] = b"validator-proposal";
pub const NAME_LEN: usize = 32;

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
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        match self {
            Config::V1(v1) => v1.validate(),
        }
    }

    pub fn as_v1(&self) -> Result<&ConfigV1> {
        match self {
            Config::V1(v1) => Ok(v1),
        }
    }

    pub fn into_v1(self) -> Result<ConfigV1> {
        match self {
            Config::V1(v1) => Ok(v1),
        }
    }
}

/// Shared V1 payload for global and validator PDAs (no mode).
///
/// `String` / `Vec` fields are dynamically sized: account bytes grow/shrink via
/// `realloc_to_fit` on every init/update (no fixed `max_len` caps).
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

    pub fn validate(&self) -> Result<()> {
        for entry in &self.block_engine.sets {
            validate_urls(entry.url.iter().map(|c| c.url.as_str()))?;
        }
        for entry in &self.p2c.sets {
            validate_urls(entry.url.iter().map(|c| c.url.as_str()))?;
        }
        Ok(())
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
    pub value: f64,
}

/// Client-side merge: validator overrides / extends global.
/// Entries are unioned by `name` (validator wins on conflict).
pub fn union_configs(global: &Config, validator: &Config) -> Result<Config> {
    let global = global.as_v1()?;
    let validator = validator.as_v1()?;
    Ok(Config::V1(ConfigV1 {
        block_engine: BlockEngineV1 {
            sets: union_by_name_be(&global.block_engine.sets, &validator.block_engine.sets),
        },
        p2c: P2cV1 {
            sets: union_by_name_p2c(&global.p2c.sets, &validator.p2c.sets),
        },
        virtual_priority: VirtualPriorityV1 {
            sets: union_by_name_vp(
                &global.virtual_priority.sets,
                &validator.virtual_priority.sets,
            ),
        },
    }))
}

fn union_by_name_be(
    global: &[BlockEngineEntryV1],
    validator: &[BlockEngineEntryV1],
) -> Vec<BlockEngineEntryV1> {
    let mut out = global.to_vec();
    for v in validator {
        if let Some(i) = out.iter().position(|g| g.name == v.name) {
            out[i] = v.clone();
        } else {
            out.push(v.clone());
        }
    }
    out
}

fn union_by_name_p2c(global: &[P2cEntryV1], validator: &[P2cEntryV1]) -> Vec<P2cEntryV1> {
    let mut out = global.to_vec();
    for v in validator {
        if let Some(i) = out.iter().position(|g| g.name == v.name) {
            out[i] = v.clone();
        } else {
            out.push(v.clone());
        }
    }
    out
}

fn union_by_name_vp(
    global: &[VirtualPriorityEntryV1],
    validator: &[VirtualPriorityEntryV1],
) -> Vec<VirtualPriorityEntryV1> {
    let mut out = global.to_vec();
    for v in validator {
        if let Some(i) = out.iter().position(|g| g.name == v.name) {
            out[i] = v.clone();
        } else {
            out.push(v.clone());
        }
    }
    out
}

#[account]
pub struct GlobalConfig {
    pub manager: Pubkey,
    pub bump: u8,
    pub config: Config,
}

#[account]
pub struct ValidatorConfig {
    pub manager: Pubkey,
    /// Key allowed to create/update the proposal PDA for this vote.
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub bump: u8,
    pub config: Config,
}

/// Per-vote draft config. Live clients ignore this until manager approve.
#[account]
pub struct ValidatorProposal {
    pub vote: Pubkey,
    /// Operator who proposed; receives rent on approve/reject close.
    pub operator: Pubkey,
    pub bump: u8,
    pub config: Config,
}

impl GlobalConfig {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(manager: Pubkey, config: &Config) -> Result<usize> {
        Self::serialized_len(&Self {
            manager,
            bump: 0,
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
        config: &Config,
    ) -> Result<usize> {
        Self::serialized_len(&Self {
            manager,
            operator,
            vote,
            bump: 0,
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

impl ValidatorProposal {
    pub fn serialized_len(account: &Self) -> Result<usize> {
        Ok(8 + account.try_to_vec()?.len())
    }

    pub fn init_space(vote: Pubkey, operator: Pubkey, config: &Config) -> Result<usize> {
        Self::serialized_len(&Self {
            vote,
            operator,
            bump: 0,
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

fn validate_urls<'a>(urls: impl Iterator<Item = &'a str>) -> Result<()> {
    for url in urls {
        require!(!url.is_empty(), crate::ConfigError::UrlEmpty);
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
