use {
    clap::{Args, Parser, Subcommand},
    colored::*,
    rakurai_activation::sdk::{
        derive_activation_account_address, derive_config_account_address,
        instruction::{
            close_rakurai_activation_account_ix, initialize_ix,
            initialize_rakurai_activation_account_ix, update_rakurai_activation_approval_ix,
            update_rakurai_activation_commission_ix, CloseRakuraiActivationAccountArgs,
            CloseRakuraiActivationAccounts, InitializeAccounts, InitializeArgs,
            InitializeRakuraiActivationAccountAccounts, InitializeRakuraiActivationAccountArgs,
            UpdateRakuraiActivationApprovalAccounts, UpdateRakuraiActivationApprovalArgs,
            UpdateRakuraiActivationCommissionAccounts, UpdateRakuraiActivationCommissionArgs,
        },
    },
    rakurai_cli::{
        display_activation_account, display_activation_config_account, get_activation_account,
        get_activation_config_account, get_node_pubkey_from_vote_account,
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, reconfirm_commission,
        sign_and_send_transaction, validate_commission, MAX_COMMISSION_BPS,
    },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
    },
    std::sync::Arc,
};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A comprehensive CLI tool for managing Rakurai Activation Account (RAA)",
    arg_required_else_help = true,
    color = clap::ColorChoice::Always
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the keypair file (must be a valid Solana keypair)
    #[arg(
        short,
        long,
        global = true,
        default_value = "~/.config/solana/id.json",
        help = "Path to the Solana keypair"
    )]
    pub keypair: String,

    /// RPC URL for sending transactions
    #[arg(short, long, global = true, default_value = "t", value_parser = normalize_to_url_if_moniker, help = "Solana RPC endpoint to send transactions through")]
    pub url: String,

    /// Rakurai Activation Program ID (Pubkey)
    #[arg(
            short,
            long,
            required = true,
            value_parser = parse_pubkey,
            help = "Rakurai activation Program ID [testnet: pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt, mainnet-beta: rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi]"
        )]
    pub program_id: Pubkey,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize Rakurai Activation config account
    #[command(hide = true)]
    InitConfig(InitConfigArgs),

    /// Display Activation config account info
    #[command(hide = true)]
    ShowConfig,

    /// Initialize a Rakurai Activation Account
    Init(InitArgs),

    /// Enable/Disable the Scheduler
    SchedulerControl(SchedulerControlArgs),

    /// Update the Validator Commission
    UpdateCommission(UpdateCommissionArgs),

    /// Close the Rakurai Activation Account
    #[command(hide = true)]
    Close(ClosePdaArgs),

    /// Display Rakurai Activation Account Info
    Show(ShowPdaArgs),
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = false, color = clap::ColorChoice::Always)]
pub struct InitConfigArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", required = true, value_parser = validate_commission, help = "Client commission percentage in base points")]
    pub client_commission_bps: Option<u16>,

    /// Client commission account pubkey
    #[arg(short = 'a', long = "commission_account", required = true, value_parser = parse_pubkey, help = "Client commission account pubkey")]
    pub client_commission_account: Option<Pubkey>,

    /// Client authority pubkey
    #[arg(short = 'b', long = "authority", required = true, value_parser = parse_pubkey, help = "Client activation authority pubkey")]
    pub client_authority: Option<Pubkey>,

    /// Config authority pubkey
    #[arg(short = 'x', long = "config_authority", required = true, value_parser = parse_pubkey, help = "Config account authority pubkey")]
    pub config_authority: Option<Pubkey>,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct InitArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "block_reward_commission_bps", alias = "commission_bps", required = false, default_value_t = MAX_COMMISSION_BPS, value_parser = validate_commission, help = "Validator commission percentage on block rewards in base points (default: 10000 bps = 100%)")]
    pub block_reward_commission_bps: u16,

    /// Validator vote account pubkey
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account pubkey")]
    pub vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct SchedulerControlArgs {
    /// Disable the scheduler if the flag is present (default: Enable)
    #[arg(
        short = 'd',
        long = "disable_scheduler",
        default_value_t = true,
        default_missing_value = "false",
        num_args = 0,
        help = "Disable the scheduler if the flag is present (default: Enable)",
        conflicts_with = "hash"
    )]
    pub disable_scheduler: bool,

    /// Optionally provide a hash value (default: None)
    #[arg(
        short = 's',
        long = "hash",
        required = false,
        help = "Optionally provide a hash value (default: None)",
        conflicts_with = "disable_scheduler"
    )]
    pub hash: Option<String>,

    /// Validator identity account pubkey
    #[arg(short = 'i', long = "identity_pubkey", required = true, value_parser = parse_pubkey, help = "Validator identity account pubkey")]
    pub identity_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct UpdateCommissionArgs {
    /// New commission value in base points (0 to 10,000). If omitted, no change is made.
    #[arg(short = 'c', long = "block_reward_commission_bps", value_parser = validate_commission, alias = "commission_bps", help = "Commission percentage on block rewards in base points (i.e: 100 bps = 1%)")]
    pub block_reward_commission_bps: u16,

    /// Validator identity account pubkey
    #[arg(short = 'i', long = "identity_pubkey", required = true, value_parser = parse_pubkey, help = "Validator identity account pubkey")]
    pub identity_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct ClosePdaArgs {
    /// Validator identity account pubkey
    #[arg(short = 'i', long = "identity_pubkey", required = true, value_parser = parse_pubkey, help = "Validator identity account pubkey")]
    pub identity_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct ShowPdaArgs {
    /// Validator identity account pubkey
    #[arg(short = 'i', long = "identity_pubkey", required = true, value_parser = parse_pubkey, help = "Validator identity account pubkey")]
    pub identity_pubkey: Pubkey,
}

fn process_init_config(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    args: InitConfigArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();

    let config_authority = args.config_authority.unwrap_or(signer_pubkey);
    let client_authority = args.client_authority.unwrap_or(signer_pubkey);
    let client_commission_bps = args.client_commission_bps.unwrap_or(1000);
    let client_commission_account = args
        .client_commission_account
        .unwrap_or(signer_pubkey);

    let (activation_config_pubkey, bump) = derive_config_account_address(&program_id);
    println!(
        "📌 Derived Config Account: {} (Bump: {})",
        activation_config_pubkey, bump
    );

    println!(
        "{} {}\n{} {}\n{} {}\n{} {}",
        "🚀 Client commission:".green(),
        client_commission_bps,
        "🏦 Commission Account:".blue(),
        client_commission_account,
        "🔑 Authority:".purple(),
        client_authority,
        "🔗 Signer and Config Authority:".cyan(),
        signer_pubkey
    );

    let initialize_instruction = initialize_ix(
        program_id,
        InitializeArgs {
            authority: config_authority,
            client_commission_bps,
            client_commission_account,
            client_authority,
            bump,
        },
        InitializeAccounts {
            config: activation_config_pubkey,
            system_program: system_program::id(),
            initializer: signer_pubkey,
        },
    );

    sign_and_send_transaction(rpc_client, initialize_instruction, &kp)
}

fn process_show_config(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let (activation_config_pubkey, _) = derive_config_account_address(&program_id);

    let activation_account =
        get_activation_config_account(rpc_client.clone(), activation_config_pubkey)?;
    println!("📌 Config Account: {}", activation_config_pubkey);
    display_activation_config_account(activation_account);
    Ok(())
}

fn process_init_pda(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    args: InitArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let block_reward_commission_bps = args.block_reward_commission_bps;
    let vote_pubkey = args.vote_pubkey;

    let node_pubkey = get_node_pubkey_from_vote_account(rpc_client.clone(), vote_pubkey)?;
    if node_pubkey != signer_pubkey {
        return Err(format!(
            "❌ Unauthorized signer! Expected: {:?}, Found: {:?}",
            node_pubkey, signer_pubkey
        )
        .into());
    }

    let (activation_config_pubkey, _) = derive_config_account_address(&program_id);
    let (activation_pubkey, bump) = derive_activation_account_address(&program_id, &node_pubkey);

    println!(
        "📌 {}",
        "Rakurai Activation Account".bold().underline().blue()
    );
    println!(
        "   🔗 Pubkey: {}",
        activation_pubkey.to_string().bold().green()
    );
    println!(
        "\n{}\n  {} {} ({}%) -> {}\n  {} {}\n  {} {}",
        "📝 Validator Configuration".bold().white(),
        "💰 Commission:".green(),
        format!("{} bps", block_reward_commission_bps),
        (block_reward_commission_bps as f64 / 100.0),
        if block_reward_commission_bps == MAX_COMMISSION_BPS {
            "Validator will keep 100% of the block rewards. No rewards will be distributed to stakers."
                .green()
                .to_string()
        } else {
            "".to_string()
        },
        "🏦 Vote Pubkey:".blue(),
        vote_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey,
    );
    if block_reward_commission_bps < MAX_COMMISSION_BPS {
        reconfirm_commission(block_reward_commission_bps)?;
    }

    let initialize_instruction = initialize_rakurai_activation_account_ix(
        program_id,
        InitializeRakuraiActivationAccountArgs {
            block_reward_commission_bps,
            bump,
        },
        InitializeRakuraiActivationAccountAccounts {
            config: activation_config_pubkey,
            system_program: system_program::id(),
            validator_vote_account: vote_pubkey,
            validator_identity_account: node_pubkey,
            activation_account: activation_pubkey,
            signer: signer_pubkey,
        },
    );

    sign_and_send_transaction(rpc_client.clone(), initialize_instruction, &kp)
}

pub fn process_scheduler_control(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    args: SchedulerControlArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();

    let disable_scheduler = args.disable_scheduler;
    let identity_pubkey = args.identity_pubkey;

    let (activation_config_pubkey, _) = derive_config_account_address(&program_id);
    let activation_config_account =
        get_activation_config_account(rpc_client.clone(), activation_config_pubkey)?;
    let (activation_pubkey, _bump) =
        derive_activation_account_address(&program_id, &identity_pubkey);
    let activation_account = get_activation_account(rpc_client.clone(), activation_pubkey)?;
    if !(identity_pubkey == signer_pubkey
        || activation_config_account.client_authority == signer_pubkey)
    {
        return Err(format!(
            "❌ Unauthorized Signer! Expected: Validator({}) or Client({}), Found: {}",
            identity_pubkey, activation_config_account.client_authority, signer_pubkey
        )
        .into());
    }

    if activation_account.is_enabled == false && disable_scheduler == false {
        return Err("Scheduler already disabled | No nedd to diable/update hash".into());
    }

    println!(
        "📌 {}",
        "Rakurai Activation Account".bold().underline().blue()
    );
    println!(
        "   🔗 Pubkey: {}",
        activation_pubkey.to_string().bold().green()
    );
    println!(
        "{} {}\n{} {}\n{} {}",
        "🚀 Scheduler Enabled:".green(),
        disable_scheduler,
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );

    let hash: Option<[u8; 64]> = if let Some(hash_str) = args.hash {
        let bytes: [u8; 64] = bs58::decode(&hash_str)
            .into_vec()
            .expect("Invalid base58")
            .try_into()
            .expect("Expected 64 bytes");

        Some(bytes)
    } else {
        None
    };

    let update_approval_instruction = update_rakurai_activation_approval_ix(
        program_id,
        UpdateRakuraiActivationApprovalArgs {
            grant_approval: disable_scheduler,
            hash,
        },
        UpdateRakuraiActivationApprovalAccounts {
            config: activation_config_pubkey,
            validator_identity_account: identity_pubkey,
            activation_account: activation_pubkey,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_approval_instruction, &kp)
}

fn process_update_commission(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    args: UpdateCommissionArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let block_reward_commission_bps = args.block_reward_commission_bps;
    let identity_pubkey = args.identity_pubkey;

    let (activation_config_pubkey, _) = derive_config_account_address(&program_id);
    let activation_config_account =
        get_activation_config_account(rpc_client.clone(), activation_config_pubkey)?;
    let (activation_pubkey, _bump) =
        derive_activation_account_address(&program_id, &identity_pubkey);
    let activation_account = get_activation_account(rpc_client.clone(), activation_pubkey)?;

    println!(
        "📌 {}",
        "Rakurai Activation Account".bold().underline().blue()
    );
    println!(
        "   🔗 Pubkey: {}",
        activation_pubkey.to_string().bold().green()
    );
    println!(
        "{} {} ({}%) {}\n{} {}\n{} {}",
        "💰 Commission BPS:".green(),
        format!("{} bps", block_reward_commission_bps),
        (block_reward_commission_bps as f64 / 100.0),
        if signer_pubkey != activation_config_account.client_authority {
            if block_reward_commission_bps == MAX_COMMISSION_BPS {
                "Validator will keep 100% of the block rewards. No rewards will be distributed to stakers."
                .green()
                .to_string()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        },
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );

    if !(signer_pubkey == activation_account.validator_authority
        || signer_pubkey == activation_config_account.client_authority)
    {
        return Err(format!(
            "❌ Unauthorized Signer! Expected: Validator({}) or Client({}), Found: {}",
            identity_pubkey, activation_config_account.client_authority, signer_pubkey
        )
        .into());
    }
    if signer_pubkey == activation_account.validator_authority
        && block_reward_commission_bps == activation_account.block_reward_commission_bps
        || signer_pubkey == activation_config_account.client_authority
            && block_reward_commission_bps == activation_account.client_commission_bps
    {
        return Err(format!("❌ No transaction required, commission value is unchanged.").into());
    }
    if signer_pubkey == activation_account.validator_authority
        && block_reward_commission_bps < MAX_COMMISSION_BPS
    {
        reconfirm_commission(block_reward_commission_bps)?;
    }

    let update_commission_instruction = update_rakurai_activation_commission_ix(
        program_id,
        UpdateRakuraiActivationCommissionArgs {
            commission_bps: block_reward_commission_bps,
        },
        UpdateRakuraiActivationCommissionAccounts {
            config: activation_config_pubkey,
            validator_identity_account: identity_pubkey,
            activation_account: activation_pubkey,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_commission_instruction, &kp)
}

fn process_close(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    program_id: Pubkey,
    args: ClosePdaArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let identity_pubkey = args.identity_pubkey;

    let (activation_config_pubkey, _) = derive_config_account_address(&program_id);
    let activation_config_account =
        get_activation_config_account(rpc_client.clone(), activation_config_pubkey)?;
    let (activation_pubkey, _bump) =
        derive_activation_account_address(&program_id, &identity_pubkey);

    if activation_config_account.client_authority != signer_pubkey {
        return Err(format!(
            "❌ Unauthorized Signer! Expected: Client({}), Found: {}",
            activation_config_account.client_authority, signer_pubkey
        )
        .into());
    }

    println!(
        "📌 {}",
        "Rakurai Activation Account".bold().underline().blue()
    );
    println!(
        "   🔗 Pubkey: {}",
        activation_pubkey.to_string().bold().green()
    );
    println!(
        "{} {}\n{} {}",
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );
    let update_approval_instruction = close_rakurai_activation_account_ix(
        program_id,
        CloseRakuraiActivationAccountArgs {},
        CloseRakuraiActivationAccounts {
            config: activation_config_pubkey,
            validator_identity_account: identity_pubkey,
            activation_account: activation_pubkey,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_approval_instruction, &kp)
}

fn process_show(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    args: ShowPdaArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity_pubkey = args.identity_pubkey;

    let (activation_pubkey, _) = derive_activation_account_address(&program_id, &identity_pubkey);

    let activation_account = get_activation_account(rpc_client.clone(), activation_pubkey)?;
    println!(
        "📌 {}",
        "Rakurai Activation Account".bold().underline().blue()
    );
    println!(
        "   🔗 Pubkey: {}",
        activation_pubkey.to_string().bold().green()
    );
    display_activation_account(activation_account);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let keypair = parse_keypair(&cli.keypair)?;
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.url.clone(),
        CommitmentConfig::confirmed(),
    ));

    match &cli.command {
        Commands::InitConfig(args) => {
            process_init_config(rpc_client.clone(), keypair, cli.program_id, args.clone())?
        }
        Commands::ShowConfig => process_show_config(rpc_client.clone(), cli.program_id)?,
        Commands::Init(args) => {
            process_init_pda(rpc_client.clone(), keypair, cli.program_id, args.clone())?
        }
        Commands::SchedulerControl(args) => {
            process_scheduler_control(rpc_client.clone(), keypair, cli.program_id, args.clone())?
        }
        Commands::UpdateCommission(args) => {
            process_update_commission(rpc_client.clone(), keypair, cli.program_id, args.clone())?
        }
        Commands::Close(args) => {
            process_close(rpc_client.clone(), keypair, cli.program_id, args.clone())?
        }
        Commands::Show(args) => process_show(rpc_client.clone(), cli.program_id, args.clone())?,
    }

    Ok(())
}
