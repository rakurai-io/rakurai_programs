//! Helpers for the `rakurai-client-config` CLI.

use {
    anchor_lang::AccountDeserialize,
    colored::*,
    rakurai_client_config::{
        sdk::{
            effective_config, name_from_str, BlockEngineConfig, BlockEngineEntryV4, BlockEngineV4,
            BundlesConfig, Config, ConfigLimits, ConfigV4, P2cEntryV4, P2cV4, SchedConfig, Uuid,
            ValidatorProposal,
            VirtualPriorityConfig, VirtualPriorityEntryV1, VirtualPriorityV1,
        },
        state::{GlobalConfig, ValidatorConfig},
    },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::pubkey::Pubkey,
    std::{fs, path::Path, str::FromStr, sync::Arc},
};

use crate::parse_pubkey;

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigFile {
    #[serde(default)]
    block_engine: SetsFileBe,
    #[serde(default)]
    p2c: SetsFileP2c,
    #[serde(default)]
    virtual_priority: SetsFileVp,
    #[serde(default)]
    sch_config: SchedConfigFile,
}

/// Missing fields take the on-chain defaults from [`SchedConfig::default`].
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct SchedConfigFile {
    rs_mode: u8,
    rs_enforce: bool,
}

impl Default for SchedConfigFile {
    fn default() -> Self {
        let d = SchedConfig::default();
        Self {
            rs_mode: d.rs_mode,
            rs_enforce: d.rs_enforce,
        }
    }
}

/// Missing fields take the on-chain defaults from [`BundlesConfig::default`].
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct BundlesConfigFile {
    bs_pm: u16,
    bs_np: u16,
    bs_be: u16,
}

impl Default for BundlesConfigFile {
    fn default() -> Self {
        let d = BundlesConfig::default();
        Self {
            bs_pm: d.bs_pm,
            bs_np: d.bs_np,
            bs_be: d.bs_be,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SetsFileBe {
    #[serde(default)]
    sets: Vec<BeEntryFile>,
    #[serde(default)]
    config: BundlesConfigFile,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BeEntryFile {
    name: String,
    url: BeUrlFile,
    #[serde(default)]
    consider_primary: bool,
    #[serde(default)]
    sch: u8,
    #[serde(default)]
    consider: u8,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BeUrlFile {
    url: String,
    #[serde(default)]
    max_bundles: u32,
    #[serde(default)]
    period_ms: u32,
    #[serde(default)]
    max_bundle_burst: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SetsFileP2c {
    #[serde(default)]
    sets: Vec<P2cEntryFile>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct P2cEntryFile {
    name: String,
    url: String,
    #[serde(default)]
    mev: bool,
    #[serde(default)]
    resell: bool,
    #[serde(default)]
    enable_tpu_p2c_update: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SetsFileVp {
    #[serde(default)]
    sets: Vec<VpEntryFile>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VpEntryFile {
    name: String,
    #[serde(default)]
    url: Vec<VpUrlFile>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VpUrlFile {
    key: String,
    value: f64,
}

pub fn uuid_to_string(uuid: &Uuid) -> String {
    let bytes = uuid.as_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub fn load_config_from_file(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(Path::new(path))?;
    let file: ConfigFile = serde_json::from_str(&text)?;
    Ok(Config::V4(ConfigV4 {
        block_engine: BlockEngineV4 {
            sets: file
                .block_engine
                .sets
                .into_iter()
                .map(|e| BlockEngineEntryV4 {
                    name: name_from_str(&e.name),
                    url: BlockEngineConfig {
                        url: e.url.url,
                        max_bundles: e.url.max_bundles,
                        period_ms: e.url.period_ms,
                        max_bundle_burst: e.url.max_bundle_burst,
                    },
                    consider_primary: e.consider_primary,
                    sch: e.sch,
                    consider: e.consider,
                })
                .collect(),
            config: BundlesConfig {
                bs_pm: file.block_engine.config.bs_pm,
                bs_np: file.block_engine.config.bs_np,
                bs_be: file.block_engine.config.bs_be,
            },
        },
        p2c: P2cV4 {
            sets: file
                .p2c
                .sets
                .into_iter()
                .map(|e| P2cEntryV4 {
                    name: name_from_str(&e.name),
                    url: e.url,
                    mev: e.mev,
                    resell: e.resell,
                    enable_tpu_p2c_update: e.enable_tpu_p2c_update,
                })
                .collect(),
        },
        virtual_priority: VirtualPriorityV1 {
            sets: file
                .virtual_priority
                .sets
                .into_iter()
                .map(
                    |e| -> Result<VirtualPriorityEntryV1, Box<dyn std::error::Error>> {
                        let mut urls = Vec::with_capacity(e.url.len());
                        for u in e.url {
                            urls.push(VirtualPriorityConfig {
                                key: Pubkey::from_str(u.key.trim())
                                    .map_err(|_| format!("Invalid pubkey: {}", u.key))?,
                                value: u.value,
                            });
                        }
                        Ok(VirtualPriorityEntryV1 {
                            name: name_from_str(&e.name),
                            url: urls,
                        })
                    },
                )
                .collect::<Result<Vec<_>, _>>()?,
        },
        sch_config: SchedConfig {
            rs_mode: file.sch_config.rs_mode,
            rs_enforce: file.sch_config.rs_enforce,
        },
    }))
}

fn config_to_file(cfg: &Config) -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let v4 = cfg
        .to_v4()
        .map_err(|_| "unsupported config schema (reserved v1); migrate to V4 before dumping")?;
    Ok(ConfigFile {
        block_engine: SetsFileBe {
            sets: v4
                .block_engine
                .sets
                .iter()
                .map(|e| BeEntryFile {
                    name: uuid_to_string(&e.name),
                    url: BeUrlFile {
                        url: e.url.url.clone(),
                        max_bundles: e.url.max_bundles,
                        period_ms: e.url.period_ms,
                        max_bundle_burst: e.url.max_bundle_burst,
                    },
                    consider_primary: e.consider_primary,
                    sch: e.sch,
                    consider: e.consider,
                })
                .collect(),
            config: BundlesConfigFile {
                bs_pm: v4.block_engine.config.bs_pm,
                bs_np: v4.block_engine.config.bs_np,
                bs_be: v4.block_engine.config.bs_be,
            },
        },
        p2c: SetsFileP2c {
            sets: v4
                .p2c
                .sets
                .iter()
                .map(|e| P2cEntryFile {
                    name: uuid_to_string(&e.name),
                    url: e.url.clone(),
                    mev: e.mev,
                    resell: e.resell,
                    enable_tpu_p2c_update: e.enable_tpu_p2c_update,
                })
                .collect(),
        },
        virtual_priority: SetsFileVp {
            sets: v4
                .virtual_priority
                .sets
                .iter()
                .map(|e| VpEntryFile {
                    name: uuid_to_string(&e.name),
                    url: e
                        .url
                        .iter()
                        .map(|u| VpUrlFile {
                            key: u.key.to_string(),
                            value: u.value,
                        })
                        .collect(),
                })
                .collect(),
        },
        sch_config: SchedConfigFile {
            rs_mode: v4.sch_config.rs_mode,
            rs_enforce: v4.sch_config.rs_enforce,
        },
    })
}

/// Write `cfg` as JSON in the same shape accepted by `load_config_from_file`.
pub fn write_config_to_file(cfg: &Config, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = config_to_file(cfg)?;
    let text = serde_json::to_string_pretty(&file)?;
    fs::write(Path::new(path), text)?;
    Ok(())
}

/// If `path` is set, dump `cfg` to that JSON file (round-trippable with update/submit).
pub fn maybe_dump_config(
    cfg: &Config,
    path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = path {
        write_config_to_file(cfg, path)?;
        println!("Wrote config to {path}");
    }
    Ok(())
}

pub fn get_global_config(
    rpc: Arc<RpcClient>,
    pda: Pubkey,
) -> Result<GlobalConfig, Box<dyn std::error::Error>> {
    let data = rpc.get_account_data(&pda)?;
    Ok(GlobalConfig::try_deserialize(&mut data.as_slice())?)
}

pub fn get_validator_config(
    rpc: Arc<RpcClient>,
    pda: Pubkey,
) -> Result<ValidatorConfig, Box<dyn std::error::Error>> {
    let data = rpc.get_account_data(&pda)?;
    Ok(ValidatorConfig::try_deserialize(&mut data.as_slice())?)
}

pub fn get_proposal(
    rpc: Arc<RpcClient>,
    pda: Pubkey,
) -> Result<ValidatorProposal, Box<dyn std::error::Error>> {
    let data = rpc.get_account_data(&pda)?;
    Ok(ValidatorProposal::try_deserialize(&mut data.as_slice())?)
}

pub fn proposal_exists(rpc: &RpcClient, pda: &Pubkey) -> bool {
    rpc.get_account_data(pda).is_ok()
}

fn display_config_payload(cfg: &Config) {
    let Ok(v4) = cfg.to_v4() else {
        println!("   schema: unsupported (reserved v1)");
        return;
    };
    let schema = match cfg {
        Config::V1 => "unsupported-v1",
        Config::V2(_) => "v2→v4",
        Config::V3(_) => "v3→v4",
        Config::V4(_) => "v4",
    };
    println!("   schema: {schema}");
    println!("   {}", "block_engine".yellow());
    let bc = &v4.block_engine.config;
    println!(
        "     config: bs_pm {} / bs_np {} / bs_be {}",
        bc.bs_pm, bc.bs_np, bc.bs_be
    );
    for entry in &v4.block_engine.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        let u = &entry.url;
        println!(
            "       {} (max_bundles {} / period_ms {} / burst {})",
            u.url, u.max_bundles, u.period_ms, u.max_bundle_burst
        );
        println!(
            "       consider_primary {} / sch {} / consider {}",
            entry.consider_primary, entry.sch, entry.consider
        );
    }
    println!("   {}", "p2c".yellow());
    for entry in &v4.p2c.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        println!(
            "       {} (mev={}, resell={}, enable_tpu_p2c_update={})",
            entry.url, entry.mev, entry.resell, entry.enable_tpu_p2c_update
        );
    }
    println!("   {}", "virtual_priority".yellow());
    for entry in &v4.virtual_priority.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        for u in &entry.url {
            println!("       {} -> {}", u.key, u.value);
        }
    }
    println!("   {}", "sch_config".yellow());
    println!(
        "     rs_mode {} / rs_enforce {}",
        v4.sch_config.rs_mode, v4.sch_config.rs_enforce
    );
}

fn display_limits(limits: &ConfigLimits) {
    match limits {
        ConfigLimits::V1(v) => println!(
            "   Limits (v1): url≤{} sets≤{} urls/set≤{} vp/set≤{}",
            v.max_url_len, v.max_sets_per_section, v.max_urls_per_set, v.max_vp_entries_per_set
        ),
    }
}

pub fn display_global_config(cfg: &GlobalConfig, pda: Pubkey) {
    let used = borsh::to_vec(&cfg).map(|v| v.len()).unwrap_or(0);
    println!("{}", "Global Validator Config".bold().underline().blue());
    println!("   PDA: {pda}");
    println!("   Manager: {}", cfg.manager);
    display_limits(&cfg.limits);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn display_validator_config(cfg: &ValidatorConfig, pda: Pubkey) {
    let used = borsh::to_vec(&cfg).map(|v| v.len()).unwrap_or(0);
    println!("{}", "Validator Config".bold().underline().blue());
    println!("   PDA: {pda}");
    println!("   Manager: {}", cfg.manager);
    println!("   Operator: {}", cfg.operator);
    println!("   Vote: {}", cfg.vote);
    display_limits(&cfg.limits);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn display_proposal(cfg: &ValidatorProposal, pda: Pubkey) {
    let used = borsh::to_vec(&cfg).map(|v| v.len()).unwrap_or(0);
    println!(
        "{}",
        "Validator Proposal (pending)".bold().underline().yellow()
    );
    println!("   PDA: {pda}");
    println!("   Vote: {}", cfg.vote);
    println!("   Operator: {}", cfg.operator);
    display_limits(&cfg.limits);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn try_get_validator_config(rpc: Arc<RpcClient>, pda: Pubkey) -> Option<ValidatorConfig> {
    get_validator_config(rpc, pda).ok()
}

pub fn display_effective(
    global: &Config,
    validator: Option<&Config>,
) -> Result<(), Box<dyn std::error::Error>> {
    let chosen = effective_config(global, validator);
    let title = if validator.is_some() {
        "Effective config (validator PDA)"
    } else {
        "Effective config (global; no validator PDA)"
    };
    println!("{}", title.bold().underline().blue());
    display_config_payload(chosen);
    Ok(())
}

pub fn parse_vote(s: &str) -> Result<Pubkey, String> {
    parse_pubkey(s)
}
