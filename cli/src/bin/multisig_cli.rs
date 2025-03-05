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
    #[arg(short, long, default_value = "/home/harkos/.config/solana/id.json", value_parser = parse_keypair, help = "Path to the Solana keypair file used for signing transactions")]
    keypair: Arc<Keypair>, 

    /// RPC URL for sending transactions
    #[arg(short, long, default_value = "l", value_parser = normalize_to_url_if_moniker, help = "Solana RPC endpoint to send transactions through")]
    rpc: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new PDA with a specified commission rate and validator vote account
    Init(InitArgs),

    /// Enable or disable the scheduler for commission approvals
    SchedulerControl(SchedulerControlArgs),

    /// Update the commission value for a validator
    UpdateCommission(UpdateCommissionArgs),

    /// Close the commission process, finalizing and deactivating any ongoing commission management
    Close,
}

#[derive(Args)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct InitArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission", required = true, value_parser = validate_commission, help = "Initial commission percentage in base points (e.g., 500 for 5%)")]
    commission: u64,

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
    #[arg(short = 'c', long = "commission", value_parser = validate_commission, help = "New commission value in base points (e.g., 500 for 5%)")]
    validator_commission: Option<u64>,
}

fn process_init(rpc: String,args: InitArgs) {
    println!(
        "{} {} (Validator Vote Pubkey: {}) {}",
        "🚀 Initializing with commission:".green(),
        args.commission,
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
    if let Some(commission) = args.validator_commission {
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
        Commands::Init(args) => process_init(cli.rpc, args),
        Commands::SchedulerControl(args) => process_scheduler_control(args),
        Commands::UpdateCommission(args) => process_update_commission(args),
        Commands::Close => process_close(),
    }
}