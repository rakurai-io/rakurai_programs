use {
    anchor_lang::{prelude::Pubkey as AnchorPubkey, AnchorSerialize},
    clap::{Args, Parser, Subcommand},
    rakurai_cli::{
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, sign_and_send_instructions,
        sign_and_send_transaction,
        validator::{
            display_effective, display_global_config, display_proposal, display_validator_config,
            get_global_config, get_proposal, get_validator_config, load_config_from_file,
            parse_vote, proposal_exists, try_get_validator_config,
        },
    },
    rakurai_client_config::sdk::{
        derive_global_config_address, derive_global_staging_address,
        derive_proposal_staging_address, derive_validator_config_address,
        derive_validator_proposal_address, derive_validator_staging_address,
        instruction::{
            abort_global_staging_ix, abort_proposal_staging_ix, abort_validator_staging_ix,
            approve_proposal_ix, close_global_ix, close_validator_ix, commit_global_staging_ix,
            commit_proposal_staging_ix, commit_validator_staging_ix, init_global_ix,
            init_global_staging_ix, init_proposal_ix, init_proposal_staging_ix, init_validator_ix,
            init_validator_staging_ix, migrate_global_to_v2_ix, migrate_proposal_to_v2_ix,
            migrate_validator_to_v2_ix, reject_proposal_ix, set_operator_ix, update_global_ix,
            update_global_limits_ix, update_proposal_ix, update_validator_ix,
            update_validator_limits_ix, write_global_staging_ix, write_proposal_staging_ix,
            write_validator_staging_ix, AbortGlobalStagingAccounts, AbortProposalStagingAccounts,
            AbortValidatorStagingAccounts, ApproveProposalAccounts, CloseGlobalAccounts,
            CloseValidatorAccounts, GlobalStagingAccounts, InitGlobalAccounts, InitGlobalArgs,
            InitProposalAccounts, InitProposalArgs, InitValidatorAccounts, InitValidatorArgs,
            ProposalStagingAccounts, RejectProposalAccounts, SetOperatorAccounts, SetOperatorArgs,
            StagingChunkArgs, StagingLenArgs, UpdateGlobalAccounts, UpdateGlobalArgs,
            UpdateGlobalLimitsAccounts, UpdateGlobalLimitsArgs, UpdateProposalAccounts,
            UpdateProposalArgs, UpdateValidatorAccounts, UpdateValidatorArgs,
            UpdateValidatorLimitsAccounts, UpdateValidatorLimitsArgs, ValidatorStagingAccounts,
        },
        Config, ConfigLimits, ConfigLimitsV1, ConfigV2,
    },
    solana_commitment_config::CommitmentConfig,
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_system_interface::program as system_program,
    std::sync::Arc,
};

const DEFAULT_PROGRAM_ID: &str = "FcTL7Mnq1RcstcYUk39ph2DzdVPNFyWh1EnrqCocXhhh";
/// Borsh payload larger than this uses staging (legacy tx packet ~1232 raw bytes).
const DIRECT_UPDATE_MAX_BYTES: usize = 800;
const STAGING_CHUNK_BYTES: usize = 640;

fn to_anchor(pubkey: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(pubkey.as_array().clone())
}

fn from_anchor(pubkey: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(pubkey.to_bytes())
}

fn global_pda(program_id: AnchorPubkey) -> Pubkey {
    from_anchor(derive_global_config_address(&program_id).0)
}

fn validator_pda(program_id: AnchorPubkey, vote: Pubkey) -> Pubkey {
    from_anchor(derive_validator_config_address(&program_id, &to_anchor(vote)).0)
}

fn proposal_pda(program_id: AnchorPubkey, vote: Pubkey) -> Pubkey {
    from_anchor(derive_validator_proposal_address(&program_id, &to_anchor(vote)).0)
}

fn global_staging_pda(program_id: AnchorPubkey) -> Pubkey {
    from_anchor(derive_global_staging_address(&program_id).0)
}

fn validator_staging_pda(program_id: AnchorPubkey, vote: Pubkey) -> Pubkey {
    from_anchor(derive_validator_staging_address(&program_id, &to_anchor(vote)).0)
}

fn proposal_staging_pda(program_id: AnchorPubkey, vote: Pubkey) -> Pubkey {
    from_anchor(derive_proposal_staging_address(&program_id, &to_anchor(vote)).0)
}

fn to_solana_instruction(
    mut ix: anchor_lang::solana_program::instruction::Instruction,
) -> Instruction {
    let acct_metas: Vec<AccountMeta> = ix
        .accounts
        .iter_mut()
        .map(|acct| AccountMeta {
            pubkey: Pubkey::new_from_array(acct.pubkey.to_bytes().clone()),
            is_signer: acct.is_signer,
            is_writable: acct.is_writable,
        })
        .collect();
    Instruction::new_with_bytes(
        Pubkey::new_from_array(ix.program_id.to_bytes()),
        &ix.data,
        acct_metas,
    )
}

#[derive(Parser)]
#[command(
    name = "rakurai-client-config",
    version,
    about = "Configure validator block-engine, P2C, and virtual-priority on-chain",
    long_about = "CLI for the rakurai_client_config program.\n\n\
        Configures per-validator:\n  \
        • block_engine — bundle submission endpoints and rate limits\n  \
        • p2c — post-pack confirmation (gRPC) endpoints\n  \
        • virtual_priority — account pubkey → priority multiplier\n\n\
        Account layers:\n  \
        • global — network-wide defaults (manager)\n  \
        • validator — live per-vote PDA (manager)\n  \
        • proposal — operator draft → manager approve/reject\n\n\
        Config payloads are JSON files (see cli/client_config.md)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true, default_value = "~/.config/solana/id.json")]
    keypair: String,

    #[arg(
        short,
        long,
        global = true,
        default_value = "t",
        value_parser = normalize_to_url_if_moniker
    )]
    url: String,

    #[arg(
        short,
        long,
        global = true,
        default_value = DEFAULT_PROGRAM_ID,
        value_parser = parse_pubkey
    )]
    program_id: Pubkey,
}

#[derive(Subcommand)]
enum Commands {
    /// Singleton global config PDA
    Global {
        #[command(subcommand)]
        command: GlobalCmd,
    },
    /// Per-vote live validator config PDA
    Validator {
        #[command(subcommand)]
        command: ValidatorCmd,
    },
    /// Per-vote proposal PDA (operator suggest → manager approve)
    Proposal {
        #[command(subcommand)]
        command: ProposalCmd,
    },
    /// Show effective config: validator PDA if present, otherwise global
    Union(UnionArgs),
}

#[derive(Subcommand)]
enum GlobalCmd {
    /// Create global config (signer becomes manager)
    Init(InitGlobalCliArgs),
    /// Replace global config payload (manager-only; reallocs). Large payloads use staging.
    Update(ConfigFileArgs),
    /// Update global size caps (manager-only)
    SetLimits(LimitsArgs),
    /// Rewrite V1 global payload to V2 (`enable_tpu_p2c_update = false`)
    MigrateToV2,
    /// Fetch and print global config
    Show,
    /// Close global config PDA and reclaim rent (manager-only)
    Close,
}

#[derive(Subcommand)]
enum ValidatorCmd {
    /// Create validator PDA, copying current global config + limits
    Init(ValidatorInitArgs),
    /// Replace validator overlay. Large payloads use ephemeral staging automatically.
    Update(ValidatorUpdateArgs),
    /// Update this vote's size caps (manager-only; used by proposals)
    SetLimits(ValidatorLimitsArgs),
    /// Set operator who may propose (manager-only)
    SetOperator(SetOperatorCliArgs),
    /// Rewrite V1 validator payload to V2 (`enable_tpu_p2c_update = false`)
    MigrateToV2(VoteArgs),
    /// Fetch and print validator config
    Show(VoteArgs),
    /// Close validator config PDA and reclaim rent (manager-only)
    Close(VoteArgs),
}

#[derive(Subcommand)]
enum ProposalCmd {
    /// Create or update proposal (operator keypair). Init if missing, else update.
    Submit(ProposalSubmitArgs),
    /// Rewrite V1 proposal payload to V2 (`enable_tpu_p2c_update = false`)
    MigrateToV2(VoteArgs),
    /// Fetch and print pending proposal
    Show(VoteArgs),
    /// Copy proposal → live validator config and close proposal (manager)
    Approve(VoteArgs),
    /// Close proposal without changing live config (manager)
    Reject(VoteArgs),
}

#[derive(Args)]
struct ConfigFileArgs {
    /// Path to JSON config (ConfigV2). Omit for empty sets. Missing `enable_tpu_p2c_update` defaults to false.
    #[arg(long)]
    config_file: Option<String>,
}

#[derive(Args)]
struct InitGlobalCliArgs {
    /// Path to JSON config (ConfigV2). Omit for empty sets. Missing `enable_tpu_p2c_update` defaults to false.
    #[arg(long)]
    config_file: Option<String>,
    #[arg(long, default_value_t = 256)]
    max_url_len: u16,
    #[arg(long, default_value_t = 16)]
    max_sets_per_section: u8,
    #[arg(long, default_value_t = 8)]
    max_urls_per_set: u8,
    #[arg(long, default_value_t = 64)]
    max_vp_entries_per_set: u8,
}

#[derive(Args)]
struct LimitsArgs {
    #[arg(long, default_value_t = 256)]
    max_url_len: u16,
    #[arg(long, default_value_t = 16)]
    max_sets_per_section: u8,
    #[arg(long, default_value_t = 8)]
    max_urls_per_set: u8,
    #[arg(long, default_value_t = 64)]
    max_vp_entries_per_set: u8,
}

#[derive(Args)]
struct ValidatorLimitsArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
    #[arg(long, default_value_t = 256)]
    max_url_len: u16,
    #[arg(long, default_value_t = 16)]
    max_sets_per_section: u8,
    #[arg(long, default_value_t = 8)]
    max_urls_per_set: u8,
    #[arg(long, default_value_t = 64)]
    max_vp_entries_per_set: u8,
}

#[derive(Args)]
struct VoteArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
}

#[derive(Args)]
struct ValidatorInitArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
    /// Operator pubkey allowed to propose. Defaults to manager keypair.
    #[arg(long, value_parser = parse_pubkey)]
    operator: Option<Pubkey>,
}

#[derive(Args)]
struct ValidatorUpdateArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
    #[arg(long)]
    config_file: String,
}

#[derive(Args)]
struct SetOperatorCliArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
    #[arg(long, value_parser = parse_pubkey)]
    operator: Pubkey,
}

#[derive(Args)]
struct ProposalSubmitArgs {
    #[arg(long, value_parser = parse_vote)]
    vote: Pubkey,
    #[arg(long)]
    config_file: String,
}

#[derive(Args)]
struct UnionArgs {
    /// Vote pubkey. If that validator PDA exists it is used; otherwise global.
    #[arg(long, value_parser = parse_vote)]
    vote: Option<Pubkey>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let kp = parse_keypair(&cli.keypair)?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        cli.url,
        CommitmentConfig::confirmed(),
    ));
    match cli.command {
        Commands::Global { command } => run_global(rpc, kp, cli.program_id, command),
        Commands::Validator { command } => run_validator(rpc, kp, cli.program_id, command),
        Commands::Proposal { command } => run_proposal(rpc, kp, cli.program_id, command),
        Commands::Union(a) => run_union(rpc, cli.program_id, a),
    }
}

fn limits_from_cli(
    max_url_len: u16,
    max_sets_per_section: u8,
    max_urls_per_set: u8,
    max_vp_entries_per_set: u8,
) -> ConfigLimits {
    ConfigLimits::V1(ConfigLimitsV1 {
        max_url_len,
        max_sets_per_section,
        max_urls_per_set,
        max_vp_entries_per_set,
    })
}

fn load_or_empty(path: Option<String>) -> Result<Config, Box<dyn std::error::Error>> {
    match path {
        Some(p) => load_config_from_file(&p),
        None => Ok(Config::V2(ConfigV2::empty())),
    }
}

fn serialize_config(config: &Config) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    config.serialize(&mut buf)?;
    Ok(buf)
}

fn should_use_staging(payload: &[u8]) -> bool {
    payload.len() > DIRECT_UPDATE_MAX_BYTES
}

fn send_ixs(
    rpc: Arc<RpcClient>,
    ixs: &[Instruction],
    kp: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    sign_and_send_instructions(rpc, ixs, kp)
}

fn staging_exists(rpc: &RpcClient, staging: Pubkey) -> bool {
    rpc.get_account(&staging).is_ok()
}

/// Close leftover staging before init so a prior failed upload cannot block the next one.
fn clear_global_staging_if_exists(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    manager: Pubkey,
    staging: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    if !staging_exists(&rpc, staging) {
        return Ok(());
    }
    println!("Existing global staging found at {staging} — aborting it first");
    send_ixs(
        rpc,
        &[to_solana_instruction(abort_global_staging_ix(
            to_anchor(program_id),
            AbortGlobalStagingAccounts {
                manager: to_anchor(manager),
                staging: to_anchor(staging),
            },
        ))],
        kp,
    )
}

fn clear_validator_staging_if_exists(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    manager: Pubkey,
    vote: Pubkey,
    staging: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    if !staging_exists(&rpc, staging) {
        return Ok(());
    }
    println!("Existing validator staging found at {staging} — aborting it first");
    send_ixs(
        rpc,
        &[to_solana_instruction(abort_validator_staging_ix(
            to_anchor(program_id),
            AbortValidatorStagingAccounts {
                manager: to_anchor(manager),
                vote: to_anchor(vote),
                staging: to_anchor(staging),
            },
        ))],
        kp,
    )
}

fn clear_proposal_staging_if_exists(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    operator: Pubkey,
    vote: Pubkey,
    staging: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    if !staging_exists(&rpc, staging) {
        return Ok(());
    }
    println!("Existing proposal staging found at {staging} — aborting it first");
    send_ixs(
        rpc,
        &[to_solana_instruction(abort_proposal_staging_ix(
            to_anchor(program_id),
            AbortProposalStagingAccounts {
                operator: to_anchor(operator),
                vote: to_anchor(vote),
                staging: to_anchor(staging),
            },
        ))],
        kp,
    )
}

fn publish_global_via_staging(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    manager: Pubkey,
    global: Pubkey,
    payload: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let staging = global_staging_pda(program_id_anchor);
    let accounts = GlobalStagingAccounts {
        manager: to_anchor(manager),
        global: to_anchor(global),
        staging: to_anchor(staging),
        system_program: to_anchor(system_program::id()),
    };
    println!(
        "Config payload {} bytes exceeds {} — uploading via global staging {}",
        payload.len(),
        DIRECT_UPDATE_MAX_BYTES,
        staging
    );
    clear_global_staging_if_exists(rpc.clone(), kp, program_id, manager, staging)?;
    send_ixs(
        rpc.clone(),
        &[to_solana_instruction(init_global_staging_ix(
            program_id_anchor,
            StagingLenArgs {
                expected_len: payload.len() as u32,
            },
            accounts,
        ))],
        kp,
    )?;
    for chunk in payload.chunks(STAGING_CHUNK_BYTES) {
        send_ixs(
            rpc.clone(),
            &[to_solana_instruction(write_global_staging_ix(
                program_id_anchor,
                StagingChunkArgs {
                    data: chunk.to_vec(),
                },
                accounts,
            ))],
            kp,
        )?;
    }
    send_ixs(
        rpc,
        &[to_solana_instruction(commit_global_staging_ix(
            program_id_anchor,
            accounts,
        ))],
        kp,
    )?;
    println!("Committed global staging and closed {staging}");
    Ok(())
}

fn publish_validator_via_staging(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    manager: Pubkey,
    vote: Pubkey,
    global: Pubkey,
    validator: Pubkey,
    payload: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let staging = validator_staging_pda(program_id_anchor, vote);
    let accounts = ValidatorStagingAccounts {
        manager: to_anchor(manager),
        vote: to_anchor(vote),
        global: to_anchor(global),
        validator: to_anchor(validator),
        staging: to_anchor(staging),
        system_program: to_anchor(system_program::id()),
    };
    println!(
        "Config payload {} bytes exceeds {} — uploading via validator staging {}",
        payload.len(),
        DIRECT_UPDATE_MAX_BYTES,
        staging
    );
    clear_validator_staging_if_exists(rpc.clone(), kp, program_id, manager, vote, staging)?;
    send_ixs(
        rpc.clone(),
        &[to_solana_instruction(init_validator_staging_ix(
            program_id_anchor,
            StagingLenArgs {
                expected_len: payload.len() as u32,
            },
            accounts,
        ))],
        kp,
    )?;
    for chunk in payload.chunks(STAGING_CHUNK_BYTES) {
        send_ixs(
            rpc.clone(),
            &[to_solana_instruction(write_validator_staging_ix(
                program_id_anchor,
                StagingChunkArgs {
                    data: chunk.to_vec(),
                },
                accounts,
            ))],
            kp,
        )?;
    }
    send_ixs(
        rpc,
        &[to_solana_instruction(commit_validator_staging_ix(
            program_id_anchor,
            accounts,
        ))],
        kp,
    )?;
    println!("Committed validator staging and closed {staging}");
    Ok(())
}

fn publish_proposal_via_staging(
    rpc: Arc<RpcClient>,
    kp: &Keypair,
    program_id: Pubkey,
    operator: Pubkey,
    vote: Pubkey,
    validator: Pubkey,
    proposal: Pubkey,
    payload: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let staging = proposal_staging_pda(program_id_anchor, vote);
    let accounts = ProposalStagingAccounts {
        operator: to_anchor(operator),
        vote: to_anchor(vote),
        validator: to_anchor(validator),
        proposal: to_anchor(proposal),
        staging: to_anchor(staging),
        system_program: to_anchor(system_program::id()),
    };
    println!(
        "Config payload {} bytes exceeds {} — uploading via proposal staging {}",
        payload.len(),
        DIRECT_UPDATE_MAX_BYTES,
        staging
    );
    clear_proposal_staging_if_exists(rpc.clone(), kp, program_id, operator, vote, staging)?;
    send_ixs(
        rpc.clone(),
        &[to_solana_instruction(init_proposal_staging_ix(
            program_id_anchor,
            StagingLenArgs {
                expected_len: payload.len() as u32,
            },
            accounts,
        ))],
        kp,
    )?;
    for chunk in payload.chunks(STAGING_CHUNK_BYTES) {
        send_ixs(
            rpc.clone(),
            &[to_solana_instruction(write_proposal_staging_ix(
                program_id_anchor,
                StagingChunkArgs {
                    data: chunk.to_vec(),
                },
                accounts,
            ))],
            kp,
        )?;
    }
    send_ixs(
        rpc,
        &[to_solana_instruction(commit_proposal_staging_ix(
            program_id_anchor,
            accounts,
        ))],
        kp,
    )?;
    println!("Committed proposal staging and closed {staging}");
    Ok(())
}

fn run_global(
    rpc: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    cmd: GlobalCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let global = global_pda(program_id_anchor);
    let manager = kp.pubkey();
    match cmd {
        GlobalCmd::Init(a) => {
            let config = load_or_empty(a.config_file)?;
            let limits = limits_from_cli(
                a.max_url_len,
                a.max_sets_per_section,
                a.max_urls_per_set,
                a.max_vp_entries_per_set,
            );
            let ix = to_solana_instruction(init_global_ix(
                program_id_anchor,
                InitGlobalArgs { config, limits },
                InitGlobalAccounts {
                    manager: to_anchor(manager),
                    global: to_anchor(global),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Update(a) => {
            let config = load_or_empty(a.config_file)?;
            let payload = serialize_config(&config)?;
            if should_use_staging(&payload) {
                publish_global_via_staging(rpc.clone(), &kp, program_id, manager, global, payload)?;
            } else {
                let ix = to_solana_instruction(update_global_ix(
                    program_id_anchor,
                    UpdateGlobalArgs { config },
                    UpdateGlobalAccounts {
                        manager: to_anchor(manager),
                        global: to_anchor(global),
                        system_program: to_anchor(system_program::id()),
                    },
                ));
                sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            }
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::SetLimits(a) => {
            let limits = limits_from_cli(
                a.max_url_len,
                a.max_sets_per_section,
                a.max_urls_per_set,
                a.max_vp_entries_per_set,
            );
            let ix = to_solana_instruction(update_global_limits_ix(
                program_id_anchor,
                UpdateGlobalLimitsArgs { limits },
                UpdateGlobalLimitsAccounts {
                    manager: to_anchor(manager),
                    global: to_anchor(global),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::MigrateToV2 => {
            let ix = to_solana_instruction(migrate_global_to_v2_ix(
                program_id_anchor,
                UpdateGlobalAccounts {
                    manager: to_anchor(manager),
                    global: to_anchor(global),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Show => {
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Close => {
            let ix = to_solana_instruction(close_global_ix(
                program_id_anchor,
                CloseGlobalAccounts {
                    manager: to_anchor(manager),
                    global: to_anchor(global),
                },
            ));
            sign_and_send_transaction(rpc, ix, &kp)?;
            println!("Closed global config {global}");
        }
    }
    Ok(())
}

fn run_validator(
    rpc: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    cmd: ValidatorCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let global = global_pda(program_id_anchor);
    let manager = kp.pubkey();
    match cmd {
        ValidatorCmd::Init(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let operator = a.operator.unwrap_or(manager);
            let ix = to_solana_instruction(init_validator_ix(
                program_id_anchor,
                InitValidatorArgs {
                    operator: to_anchor(operator),
                },
                InitValidatorAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Update(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let config = load_config_from_file(&a.config_file)?;
            let payload = serialize_config(&config)?;
            if should_use_staging(&payload) {
                publish_validator_via_staging(
                    rpc.clone(),
                    &kp,
                    program_id,
                    manager,
                    a.vote,
                    global,
                    validator,
                    payload,
                )?;
            } else {
                let ix = to_solana_instruction(update_validator_ix(
                    program_id_anchor,
                    UpdateValidatorArgs { config },
                    UpdateValidatorAccounts {
                        manager: to_anchor(manager),
                        vote: to_anchor(a.vote),
                        global: to_anchor(global),
                        validator: to_anchor(validator),
                        system_program: to_anchor(system_program::id()),
                    },
                ));
                sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            }
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::SetLimits(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let limits = limits_from_cli(
                a.max_url_len,
                a.max_sets_per_section,
                a.max_urls_per_set,
                a.max_vp_entries_per_set,
            );
            let ix = to_solana_instruction(update_validator_limits_ix(
                program_id_anchor,
                UpdateValidatorLimitsArgs { limits },
                UpdateValidatorLimitsAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::SetOperator(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let ix = to_solana_instruction(set_operator_ix(
                program_id_anchor,
                SetOperatorArgs {
                    operator: to_anchor(a.operator),
                },
                SetOperatorAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::MigrateToV2(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let ix = to_solana_instruction(migrate_validator_to_v2_ix(
                program_id_anchor,
                UpdateValidatorAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Show(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Close(a) => {
            let validator = validator_pda(program_id_anchor, a.vote);
            let ix = to_solana_instruction(close_validator_ix(
                program_id_anchor,
                CloseValidatorAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                },
            ));
            sign_and_send_transaction(rpc, ix, &kp)?;
            println!("Closed validator config {validator} (vote {})", a.vote);
        }
    }
    Ok(())
}

fn run_proposal(
    rpc: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    cmd: ProposalCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id_anchor = to_anchor(program_id);
    let global = global_pda(program_id_anchor);
    match cmd {
        ProposalCmd::Submit(a) => {
            let operator = kp.pubkey();
            let validator = validator_pda(program_id_anchor, a.vote);
            let proposal = proposal_pda(program_id_anchor, a.vote);
            let config = load_config_from_file(&a.config_file)?;
            let payload = serialize_config(&config)?;
            if should_use_staging(&payload) {
                if !proposal_exists(rpc.as_ref(), &proposal) {
                    let ix = to_solana_instruction(init_proposal_ix(
                        program_id_anchor,
                        InitProposalArgs {
                            config: Config::V2(ConfigV2::empty()),
                        },
                        InitProposalAccounts {
                            operator: to_anchor(operator),
                            vote: to_anchor(a.vote),
                            validator: to_anchor(validator),
                            proposal: to_anchor(proposal),
                            system_program: to_anchor(system_program::id()),
                        },
                    ));
                    sign_and_send_transaction(rpc.clone(), ix, &kp)?;
                }
                publish_proposal_via_staging(
                    rpc.clone(),
                    &kp,
                    program_id,
                    operator,
                    a.vote,
                    validator,
                    proposal,
                    payload,
                )?;
            } else if proposal_exists(rpc.as_ref(), &proposal) {
                let ix = to_solana_instruction(update_proposal_ix(
                    program_id_anchor,
                    UpdateProposalArgs { config },
                    UpdateProposalAccounts {
                        operator: to_anchor(operator),
                        vote: to_anchor(a.vote),
                        validator: to_anchor(validator),
                        proposal: to_anchor(proposal),
                        system_program: to_anchor(system_program::id()),
                    },
                ));
                sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            } else {
                let ix = to_solana_instruction(init_proposal_ix(
                    program_id_anchor,
                    InitProposalArgs { config },
                    InitProposalAccounts {
                        operator: to_anchor(operator),
                        vote: to_anchor(a.vote),
                        validator: to_anchor(validator),
                        proposal: to_anchor(proposal),
                        system_program: to_anchor(system_program::id()),
                    },
                ));
                sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            }
            display_proposal(&get_proposal(rpc, proposal)?, proposal);
        }
        ProposalCmd::MigrateToV2(a) => {
            let operator = kp.pubkey();
            let validator = validator_pda(program_id_anchor, a.vote);
            let proposal = proposal_pda(program_id_anchor, a.vote);
            let ix = to_solana_instruction(migrate_proposal_to_v2_ix(
                program_id_anchor,
                UpdateProposalAccounts {
                    operator: to_anchor(operator),
                    vote: to_anchor(a.vote),
                    validator: to_anchor(validator),
                    proposal: to_anchor(proposal),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_proposal(&get_proposal(rpc, proposal)?, proposal);
        }
        ProposalCmd::Show(a) => {
            let proposal = proposal_pda(program_id_anchor, a.vote);
            display_proposal(&get_proposal(rpc, proposal)?, proposal);
        }
        ProposalCmd::Approve(a) => {
            let manager = kp.pubkey();
            let validator = validator_pda(program_id_anchor, a.vote);
            let proposal = proposal_pda(program_id_anchor, a.vote);
            let pending = get_proposal(rpc.clone(), proposal)?;
            let ix = to_solana_instruction(approve_proposal_ix(
                program_id_anchor,
                ApproveProposalAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    operator: pending.operator,
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                    proposal: to_anchor(proposal),
                    system_program: to_anchor(system_program::id()),
                },
            ));
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
            println!("Approved proposal; proposal account closed");
        }
        ProposalCmd::Reject(a) => {
            let manager = kp.pubkey();
            let validator = validator_pda(program_id_anchor, a.vote);
            let proposal = proposal_pda(program_id_anchor, a.vote);
            let pending = get_proposal(rpc.clone(), proposal)?;
            let ix = to_solana_instruction(reject_proposal_ix(
                program_id_anchor,
                RejectProposalAccounts {
                    manager: to_anchor(manager),
                    vote: to_anchor(a.vote),
                    operator: pending.operator,
                    global: to_anchor(global),
                    validator: to_anchor(validator),
                    proposal: to_anchor(proposal),
                },
            ));
            sign_and_send_transaction(rpc, ix, &kp)?;
            println!("Rejected proposal {proposal}; live validator config unchanged");
        }
    }
    Ok(())
}

fn run_union(
    rpc: Arc<RpcClient>,
    program_id: Pubkey,
    a: UnionArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let global = global_pda(to_anchor(program_id));
    let g = get_global_config(rpc.clone(), global)?;
    let validator_cfg = a.vote.and_then(|vote| {
        let validator = validator_pda(to_anchor(program_id), vote);
        try_get_validator_config(rpc.clone(), validator).map(|c| c.config)
    });
    display_effective(&g.config, validator_cfg.as_ref())
}
