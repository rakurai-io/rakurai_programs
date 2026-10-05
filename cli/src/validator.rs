//! Helpers for the `rakurai-client-config` CLI.

use {
    anchor_lang::AccountDeserialize,
    colored::*,
    rakurai_client_config::{
        sdk::{
            effective_config, name_from_str, BlockEngineConfig, BlockEngineEntryV4, BlockEngineV4,
            Config, ConfigLimits, ConfigV4, P2cEntryV4, P2cV4, Uuid, ValidatorProposal,
            VirtualPriorityConfig, VirtualPriorityEntryV1, VirtualPriorityV1,
        },
        state::{GlobalConfig, ValidatorConfig},
    },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::pubkey::Pubkey,
    std::{fs, path::Path, str::FromStr, sync::Arc},
};

use crate::parse_pubkey;

#[derive(serde::Deserialize)]
struct ConfigFile {
    #[serde(default)]
    block_engine: SetsFileBe,
    #[serde(default)]
    p2c: SetsFileP2c,
    #[serde(default)]
    virtual_priority: SetsFileVp,
}

#[derive(serde::Deserialize, Default)]
struct SetsFileBe {
    #[serde(default)]
    sets: Vec<BeEntryFile>,
}

#[derive(serde::Deserialize)]
struct BeEntryFile {
    name: String,
    url: BeUrlFile,
}

#[derive(serde::Deserialize)]
struct BeUrlFile {
    url: String,
    #[serde(default)]
    max_bundles: u32,
    #[serde(default)]
    period_ms: u32,
    #[serde(default)]
    max_bundle_burst: u32,
}

#[derive(serde::Deserialize, Default)]
struct SetsFileP2c {
    #[serde(default)]
    sets: Vec<P2cEntryFile>,
}

#[derive(serde::Deserialize)]
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

#[derive(serde::Deserialize, Default)]
struct SetsFileVp {
    #[serde(default)]
    sets: Vec<VpEntryFile>,
}

#[derive(serde::Deserialize)]
struct VpEntryFile {
    name: String,
    #[serde(default)]
    url: Vec<VpUrlFile>,
}

#[derive(serde::Deserialize)]
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
                })
                .collect(),
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
    }))
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
    for entry in &v4.block_engine.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        let u = &entry.url;
        println!(
            "       {} (max_bundles {} / period_ms {} / burst {})",
            u.url, u.max_bundles, u.period_ms, u.max_bundle_burst
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
