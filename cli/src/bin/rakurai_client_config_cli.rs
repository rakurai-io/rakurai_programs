use {
    anchor_lang::solana_program::system_program,
    clap::{Args, Parser, Subcommand},
    rakurai_cli::{
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, sign_and_send_transaction,
        validator::{
            display_global_config, display_proposal, display_union, display_validator_config,
            get_global_config, get_proposal, get_validator_config, load_config_from_file,
            parse_vote, proposal_exists, try_get_validator_config,
        },
    },
    rakurai_client_config::sdk::{
        derive_global_config_address, derive_validator_config_address,
        derive_validator_proposal_address,
        instruction::{
            approve_proposal_ix, close_global_ix, close_validator_ix, init_global_ix,
            init_proposal_ix, init_validator_ix, reject_proposal_ix, set_operator_ix,
            update_global_ix, update_global_limits_ix, update_proposal_ix, update_validator_ix,
            update_validator_limits_ix, ApproveProposalAccounts, CloseGlobalAccounts,
            CloseValidatorAccounts, InitGlobalAccounts, InitGlobalArgs, InitProposalAccounts,
            InitProposalArgs, InitValidatorAccounts, InitValidatorArgs, RejectProposalAccounts,
            SetOperatorAccounts, SetOperatorArgs, UpdateGlobalAccounts, UpdateGlobalArgs,
            UpdateGlobalLimitsAccounts, UpdateGlobalLimitsArgs, UpdateProposalAccounts,
            UpdateProposalArgs, UpdateValidatorAccounts, UpdateValidatorArgs,
            UpdateValidatorLimitsAccounts, UpdateValidatorLimitsArgs,
        },
        Config, ConfigLimits, ConfigLimitsV1, ConfigV1,
    },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::Signer,
    },
    std::sync::Arc,
};

const DEFAULT_PROGRAM_ID: &str = "4uGNMjJFxgE3TfEiPmSpvfwYah12QZbaWWZDJqZvA9F4";

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
    /// Show client-side union of global + optional validator (by entry name; validator wins)
    Union(UnionArgs),
}

#[derive(Subcommand)]
enum GlobalCmd {
    /// Create global config (signer becomes manager)
    Init(InitGlobalCliArgs),
    /// Replace global config payload (manager-only; reallocs)
    Update(ConfigFileArgs),
    /// Update global size caps (manager-only)
    SetLimits(LimitsArgs),
    /// Fetch and print global config
    Show,
    /// Close global config PDA and reclaim rent (manager-only)
    Close,
}

#[derive(Subcommand)]
enum ValidatorCmd {
    /// Create validator PDA, copying current global config + limits
    Init(ValidatorInitArgs),
    /// Replace validator config payload (manager-only; reallocs)
    Update(ValidatorUpdateArgs),
    /// Update this vote's size caps (manager-only; used by proposals)
    SetLimits(ValidatorLimitsArgs),
    /// Set operator who may propose (manager-only)
    SetOperator(SetOperatorCliArgs),
    /// Fetch and print validator config
    Show(VoteArgs),
    /// Close validator config PDA and reclaim rent (manager-only)
    Close(VoteArgs),
}

#[derive(Subcommand)]
enum ProposalCmd {
    /// Create or update proposal (operator keypair). Init if missing, else update.
    Submit(ProposalSubmitArgs),
    /// Fetch and print pending proposal
    Show(VoteArgs),
    /// Copy proposal → live validator config and close proposal (manager)
    Approve(VoteArgs),
    /// Close proposal without changing live config (manager)
    Reject(VoteArgs),
}

#[derive(Args)]
struct ConfigFileArgs {
    /// Path to JSON config (ConfigV1). Omit for empty sets.
    #[arg(long)]
    config_file: Option<String>,
}

#[derive(Args)]
struct InitGlobalCliArgs {
    /// Path to JSON config (ConfigV1). Omit for empty sets.
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
    /// Vote pubkey for per-validator overlay. Omit if no validator PDA exists yet.
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
        None => Ok(Config::V1(ConfigV1::empty())),
    }
}

fn run_global(
    rpc: Arc<RpcClient>,
    kp: Arc<solana_sdk::signature::Keypair>,
    program_id: Pubkey,
    cmd: GlobalCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let (global, _) = derive_global_config_address(&program_id);
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
            let ix = init_global_ix(
                program_id,
                InitGlobalArgs { config, limits },
                InitGlobalAccounts {
                    manager,
                    global,
                    system_program: system_program::ID,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Update(a) => {
            let config = load_or_empty(a.config_file)?;
            let ix = update_global_ix(
                program_id,
                UpdateGlobalArgs { config },
                UpdateGlobalAccounts {
                    manager,
                    global,
                    system_program: system_program::ID,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::SetLimits(a) => {
            let limits = limits_from_cli(
                a.max_url_len,
                a.max_sets_per_section,
                a.max_urls_per_set,
                a.max_vp_entries_per_set,
            );
            let ix = update_global_limits_ix(
                program_id,
                UpdateGlobalLimitsArgs { limits },
                UpdateGlobalLimitsAccounts {
                    manager,
                    global,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Show => {
            display_global_config(&get_global_config(rpc, global)?, global);
        }
        GlobalCmd::Close => {
            let ix = close_global_ix(program_id, CloseGlobalAccounts { manager, global });
            sign_and_send_transaction(rpc, ix, &kp)?;
            println!("Closed global config {global}");
        }
    }
    Ok(())
}

fn run_validator(
    rpc: Arc<RpcClient>,
    kp: Arc<solana_sdk::signature::Keypair>,
    program_id: Pubkey,
    cmd: ValidatorCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let (global, _) = derive_global_config_address(&program_id);
    let manager = kp.pubkey();
    match cmd {
        ValidatorCmd::Init(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let operator = a.operator.unwrap_or(manager);
            let ix = init_validator_ix(
                program_id,
                InitValidatorArgs { operator },
                InitValidatorAccounts {
                    manager,
                    vote: a.vote,
                    global,
                    validator,
                    system_program: system_program::ID,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Update(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let config = load_config_from_file(&a.config_file)?;
            let ix = update_validator_ix(
                program_id,
                UpdateValidatorArgs { config },
                UpdateValidatorAccounts {
                    manager,
                    vote: a.vote,
                    global,
                    validator,
                    system_program: system_program::ID,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::SetLimits(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let limits = limits_from_cli(
                a.max_url_len,
                a.max_sets_per_section,
                a.max_urls_per_set,
                a.max_vp_entries_per_set,
            );
            let ix = update_validator_limits_ix(
                program_id,
                UpdateValidatorLimitsArgs { limits },
                UpdateValidatorLimitsAccounts {
                    manager,
                    vote: a.vote,
                    global,
                    validator,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::SetOperator(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let ix = set_operator_ix(
                program_id,
                SetOperatorArgs {
                    operator: a.operator,
                },
                SetOperatorAccounts {
                    manager,
                    vote: a.vote,
                    global,
                    validator,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Show(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
        }
        ValidatorCmd::Close(a) => {
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let ix = close_validator_ix(
                program_id,
                CloseValidatorAccounts {
                    manager,
                    vote: a.vote,
                    global,
                    validator,
                },
            );
            sign_and_send_transaction(rpc, ix, &kp)?;
            println!("Closed validator config {validator} (vote {})", a.vote);
        }
    }
    Ok(())
}

fn run_proposal(
    rpc: Arc<RpcClient>,
    kp: Arc<solana_sdk::signature::Keypair>,
    program_id: Pubkey,
    cmd: ProposalCmd,
) -> Result<(), Box<dyn std::error::Error>> {
    let (global, _) = derive_global_config_address(&program_id);
    match cmd {
        ProposalCmd::Submit(a) => {
            let operator = kp.pubkey();
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let (proposal, _) = derive_validator_proposal_address(&program_id, &a.vote);
            let config = load_config_from_file(&a.config_file)?;
            let ix = if proposal_exists(rpc.as_ref(), &proposal) {
                update_proposal_ix(
                    program_id,
                    UpdateProposalArgs { config },
                    UpdateProposalAccounts {
                        operator,
                        vote: a.vote,
                        validator,
                        proposal,
                        system_program: system_program::ID,
                    },
                )
            } else {
                init_proposal_ix(
                    program_id,
                    InitProposalArgs { config },
                    InitProposalAccounts {
                        operator,
                        vote: a.vote,
                        validator,
                        proposal,
                        system_program: system_program::ID,
                    },
                )
            };
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_proposal(&get_proposal(rpc, proposal)?, proposal);
        }
        ProposalCmd::Show(a) => {
            let (proposal, _) = derive_validator_proposal_address(&program_id, &a.vote);
            display_proposal(&get_proposal(rpc, proposal)?, proposal);
        }
        ProposalCmd::Approve(a) => {
            let manager = kp.pubkey();
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let (proposal, _) = derive_validator_proposal_address(&program_id, &a.vote);
            let pending = get_proposal(rpc.clone(), proposal)?;
            let ix = approve_proposal_ix(
                program_id,
                ApproveProposalAccounts {
                    manager,
                    vote: a.vote,
                    operator: pending.operator,
                    global,
                    validator,
                    proposal,
                    system_program: system_program::ID,
                },
            );
            sign_and_send_transaction(rpc.clone(), ix, &kp)?;
            display_validator_config(&get_validator_config(rpc, validator)?, validator);
            println!("Approved proposal; proposal account closed");
        }
        ProposalCmd::Reject(a) => {
            let manager = kp.pubkey();
            let (validator, _) = derive_validator_config_address(&program_id, &a.vote);
            let (proposal, _) = derive_validator_proposal_address(&program_id, &a.vote);
            let pending = get_proposal(rpc.clone(), proposal)?;
            let ix = reject_proposal_ix(
                program_id,
                RejectProposalAccounts {
                    manager,
                    vote: a.vote,
                    operator: pending.operator,
                    global,
                    validator,
                    proposal,
                },
            );
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
    let (global, _) = derive_global_config_address(&program_id);
    let g = get_global_config(rpc.clone(), global)?;
    let validator_cfg = a.vote.and_then(|vote| {
        let (validator, _) = derive_validator_config_address(&program_id, &vote);
        try_get_validator_config(rpc.clone(), validator).map(|c| c.config)
    });
    display_union(&g.config, validator_cfg.as_ref())
}
