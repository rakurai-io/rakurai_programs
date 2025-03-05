use {
    clap::{Args, Parser, Subcommand},
    colored::*,
    solana_sdk::{pubkey::Pubkey, signature::Keypair},
    std::sync::Arc,
    rakurai_cli::{normalize_to_url_if_moniker, parse_keypair, parse_pubkey, validate_commission},
};

#[derive(Parser)]
#[command(
    author, 
    version, 
    about = "A comprehensive CLI tool for managing commission approvals",
    arg_required_else_help = true, 
    color = clap::ColorChoice::Always
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the keypair file (must be a valid Solana keypair file for signing transactions)
    #[arg(short, long, default_value = "~/.config/solana/id.json", value_parser = parse_keypair, help = "Path to the Solana keypair file used for signing transactions")]
    keypair: Arc<Keypair>, 

    /// RPC URL for sending transactions
    #[arg(short, long, default_value = "l", value_parser = normalize_to_url_if_moniker, help = "Solana RPC endpoint to send transactions through")]
    rpc: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize block builder config account
    InitConfig(InitConfigArgs),

    /// Initialize a new PDA with a specified commission rate and validator vote account
    InitPda(InitPdaArgs),

    /// Enable or disable the scheduler for commission approvals
    SchedulerControl(SchedulerControlArgs),

    /// Update the commission value for a validator
    UpdateCommission(UpdateCommissionArgs),

    /// Close the commission process, finalizing and deactivating any ongoing commission management
    Close,
}

#[derive(Args)]
#[command(arg_required_else_help = false, color = clap::ColorChoice::Always)]
struct InitConfigArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "Block builder commission percentage in base points (e.g., 500 for 5%)")]
    block_builder_commission_bps: Option<u64>,

    /// Block builder commission account public key
    #[arg(short = 'a', long = "commission_account", value_parser = parse_pubkey, help = "Block builder commission account pubkey")]
    block_builder_commission_account: Option<Pubkey>,

    /// Block builder authority public key
    #[arg(short = 'b', long = "authority", value_parser = parse_pubkey, help = "Block builder multisig authority pubkey")]
    block_builder_authority: Option<Pubkey>,

    /// Config authority public key
    #[arg(short = 'x', long = "config_authority", value_parser = parse_pubkey, help = "Config account authority pubkey")]
    config_authority: Option<Pubkey>,
}


#[derive(Args)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct InitPdaArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", required = true, value_parser = validate_commission, help = "Initial commission percentage in base points (e.g., 500 for 5%)")]
    commission_bps: u64,

    /// Validator vote account public key
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account public key")]
    validator_vote_pubkey: Pubkey,
}

#[derive(Args)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct SchedulerControlArgs {
    /// Enable or disable the scheduler (true = enable, false = disable)
    #[arg(short = 'e', long = "enable", required = true, help = "Pass `true` to enable the scheduler, `false` to disable it")]
    enable_scheduler: bool,
}

#[derive(Args)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct UpdateCommissionArgs {
    /// New commission value in base points (0 to 10,000). If omitted, no change is made.
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "New commission value in base points (e.g., 500 for 5%)")]
    commission_bps: Option<u64>,
}

fn process_init_config(rpc: String, args: InitConfigArgs) {
    println!(
        "{} {} (Validator Vote Pubkey: {})\n{} {}\n{} {}\n{} {}",
        "🚀 Initializing with commission:".green(),
        args.block_builder_commission_bps
            .map_or("Not provided".to_string(), |v| v.to_string()),
        args.block_builder_authority
            .map_or("Not provided".to_string(), |v| v.to_string()),
        "🏦 Commission Account:".blue(),
        args.block_builder_commission_account
            .map_or("Not provided".to_string(), |v| v.to_string()),
        "🔑 Config Authority:".purple(),
        args.config_authority
            .map_or("Not provided".to_string(), |v| v.to_string()),
        "🔗 RPC Endpoint:".cyan(),
        rpc
    );
}


fn process_init_pda(rpc: String,args: InitPdaArgs) {
    println!(
        "{} {} (Validator Vote Pubkey: {}) {}",
        "🚀 Initializing with commission:".green(),
        args.commission_bps,
        args.validator_vote_pubkey,
        rpc
    );
}

fn process_scheduler_control(args: SchedulerControlArgs) {
    println!(
        "{}",
        if args.enable_scheduler {
            "✅ Scheduler Enabled".blue()
        } else {
            "❌ Scheduler Disabled".red()
        }
    );
}

fn process_update_commission(args: UpdateCommissionArgs) {
    if let Some(commission) = args.commission_bps {
        println!(
            "{} {}",
            "🔄 Updating commission to:".yellow(),
            commission
        );
    } else {
        println!("{}", "⚠️ No commission value provided. No changes made.".red());
    }
}

fn process_close() {
    println!("{}", "🔒 Closing commission process".red());
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::InitPda(args) => process_init_pda(cli.rpc, args),
        Commands::InitConfig(args) => process_init_config(cli.rpc, args),
        Commands::SchedulerControl(args) => process_scheduler_control(args),
        Commands::UpdateCommission(args) => process_update_commission(args),
        Commands::Close => process_close(),
    }
}