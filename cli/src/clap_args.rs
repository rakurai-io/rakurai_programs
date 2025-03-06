use {
    crate::{normalize_to_url_if_moniker, parse_keypair, parse_pubkey, validate_commission},
    clap::{Args, Parser, Subcommand},
    solana_sdk::{pubkey::Pubkey, signature::Keypair},
    std::sync::Arc,
};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A comprehensive CLI tool for managing rakurai multisig account",
    arg_required_else_help = true,
    color = clap::ColorChoice::Always
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the keypair file (must be a valid Solana keypair)
    #[arg(short, long, default_value = "~/.config/solana/test_identity.json", value_parser = parse_keypair, help = "Path to the Solana keypair")]
    pub keypair: Arc<Keypair>,

    /// RPC URL for sending transactions
    #[arg(short, long, default_value = "t", value_parser = normalize_to_url_if_moniker, help = "Solana RPC endpoint to send transactions through")]
    pub rpc: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize block builder config account
    #[command(hide = true)]
    InitConfig(InitConfigArgs),

    /// Initialize a new multisig account
    InitPda(InitPdaArgs),

    /// Enable or disable the scheduler
    SchedulerControl(SchedulerControlArgs),

    /// Update the validator commission
    UpdateCommission(UpdateCommissionArgs),

    /// Close the multisig account
    Close(ClosePdaArgs),
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = false, color = clap::ColorChoice::Always)]
pub struct InitConfigArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "Block builder commission percentage in base points")]
    pub block_builder_commission_bps: Option<u16>,

    /// Block builder commission account pubkey
    #[arg(short = 'a', long = "commission_account", value_parser = parse_pubkey, help = "Block builder commission account pubkey")]
    pub block_builder_commission_account: Option<Pubkey>,

    /// Block builder authority pubkey
    #[arg(short = 'b', long = "authority", value_parser = parse_pubkey, help = "Block builder multisig authority pubkey")]
    pub block_builder_authority: Option<Pubkey>,

    /// Config authority pubkey
    #[arg(short = 'x', long = "config_authority", value_parser = parse_pubkey, help = "Config account authority pubkey")]
    pub config_authority: Option<Pubkey>,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct InitPdaArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", required = true, value_parser = validate_commission, help = "Commission percentage in base points")]
    pub commission_bps: u16,

    /// Validator vote account pubkey
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account pubkey")]
    pub vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct SchedulerControlArgs {
    /// Disable the scheduler if the flag is present (default: Enable)
    #[arg(
        short = 'e',
        long = "disable_scheduler",
        default_value_t = true,
        default_missing_value = "false",
        num_args = 0,
        help = "Disable the scheduler if the flag is present (default: Enable)"
    )]
    pub disable_scheduler: bool,

    /// Validator vote account pubkey
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account pubkey")]
    pub vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct UpdateCommissionArgs {
    /// New commission value in base points (0 to 10,000). If omitted, no change is made.
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "New commission value in base points")]
    pub commission_bps: Option<u16>,

    /// Validator vote account pubkey
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account pubkey")]
    pub vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
pub struct ClosePdaArgs {
    /// Validator vote account pubkey
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account pubkey")]
    pub vote_pubkey: Pubkey,
}
