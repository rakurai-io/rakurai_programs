//! Helpers for the `rakurai-validator-config` CLI.

use {
    anchor_lang::{AccountDeserialize, AnchorSerialize},
    colored::*,
    rakurai_validator_config::{
        sdk::{
            name_from_str, union_configs, BlockEngineConfig, BlockEngineEntryV1, BlockEngineV1,
            Config, ConfigV1, P2cConfig, P2cEntryV1, P2cV1, Uuid, ValidatorProposal,
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
    #[serde(default)]
    url: Vec<BeUrlFile>,
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
    #[serde(default)]
    url: Vec<P2cUrlFile>,
}

#[derive(serde::Deserialize)]
struct P2cUrlFile {
    url: String,
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
    Ok(Config::V1(ConfigV1 {
        block_engine: BlockEngineV1 {
            sets: file
                .block_engine
                .sets
                .into_iter()
                .map(|e| BlockEngineEntryV1 {
                    name: name_from_str(&e.name),
                    url: e
                        .url
                        .into_iter()
                        .map(|u| BlockEngineConfig {
                            url: u.url,
                            max_bundles: u.max_bundles,
                            period_ms: u.period_ms,
                            max_bundle_burst: u.max_bundle_burst,
                        })
                        .collect(),
                })
                .collect(),
        },
        p2c: P2cV1 {
            sets: file
                .p2c
                .sets
                .into_iter()
                .map(|e| P2cEntryV1 {
                    name: name_from_str(&e.name),
                    url: e
                        .url
                        .into_iter()
                        .map(|u| P2cConfig { url: u.url })
                        .collect(),
                })
                .collect(),
        },
        virtual_priority: VirtualPriorityV1 {
            sets: file
                .virtual_priority
                .sets
                .into_iter()
                .map(|e| -> Result<VirtualPriorityEntryV1, Box<dyn std::error::Error>> {
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
                })
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
    let Config::V1(v1) = cfg;
    println!("   {}", "block_engine".yellow());
    for entry in &v1.block_engine.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        for u in &entry.url {
            println!(
                "       {} (max_bundles {} / period_ms {} / burst {})",
                u.url, u.max_bundles, u.period_ms, u.max_bundle_burst
            );
        }
    }
    println!("   {}", "p2c".yellow());
    for entry in &v1.p2c.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        for u in &entry.url {
            println!("       {}", u.url);
        }
    }
    println!("   {}", "virtual_priority".yellow());
    for entry in &v1.virtual_priority.sets {
        println!("     [{}]", uuid_to_string(&entry.name));
        for u in &entry.url {
            println!("       {} -> {}", u.key, u.value);
        }
    }
}

pub fn display_global_config(cfg: &GlobalConfig, pda: Pubkey) {
    let used = cfg.try_to_vec().map(|v| v.len()).unwrap_or(0);
    println!("{}", "Global Validator Config".bold().underline().blue());
    println!("   PDA: {pda}");
    println!("   Manager: {}", cfg.manager);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn display_validator_config(cfg: &ValidatorConfig, pda: Pubkey) {
    let used = cfg.try_to_vec().map(|v| v.len()).unwrap_or(0);
    println!("{}", "Validator Config".bold().underline().blue());
    println!("   PDA: {pda}");
    println!("   Manager: {}", cfg.manager);
    println!("   Operator: {}", cfg.operator);
    println!("   Vote: {}", cfg.vote);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn display_proposal(cfg: &ValidatorProposal, pda: Pubkey) {
    let used = cfg.try_to_vec().map(|v| v.len()).unwrap_or(0);
    println!("{}", "Validator Proposal (pending)".bold().underline().yellow());
    println!("   PDA: {pda}");
    println!("   Vote: {}", cfg.vote);
    println!("   Operator: {}", cfg.operator);
    println!("   Size: {used} bytes (+ 8 discriminator)");
    display_config_payload(&cfg.config);
}

pub fn display_union(global: &Config, validator: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let merged = union_configs(global, validator)?;
    println!(
        "{}",
        "Union (by entry name; validator wins on conflict)"
            .bold()
            .underline()
            .blue()
    );
    display_config_payload(&merged);
    Ok(())
}

pub fn parse_vote(s: &str) -> Result<Pubkey, String> {
    parse_pubkey(s)
}
