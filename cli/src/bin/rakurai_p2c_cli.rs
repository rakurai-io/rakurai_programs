use {
    anchor_lang::AccountDeserialize,
    clap::{Args, Parser, Subcommand},
    colored::{ColoredString, Colorize},
    rakurai_cli::{
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, sign_and_send_transaction,
    },
    reward_distribution::{
        sdk::{
            derive_p2c_subscription_address,
            instruction::{
                claim_epoch_p2c_subscription_ix, clear_p2c_deficit_ix, fund_p2c_subscription_ix,
                record_p2c_subscription_ix, ClaimEpochP2CSubscriptionAccounts,
                ClaimEpochP2CSubscriptionArgs, ClearP2CDeficitAccounts, ClearP2CDeficitArgs,
                FundP2CSubscriptionAccounts, FundP2CSubscriptionArgs,
                RecordP2CSubscriptionAccounts, RecordP2CSubscriptionArgs,
            },
        },
        state::{P2CSubscriptionAccount, P2CSubscriptionStatus},
    },
    solana_account_decoder_client_types::UiAccountEncoding,
    solana_rpc_client::rpc_client::RpcClient,
    solana_rpc_client_api::{
        config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer, system_program,
    },
    std::{error::Error, sync::Arc},
};

type CliResult<T = ()> = Result<T, Box<dyn Error>>;

/// P2C account name starts immediately after the 8-byte Anchor discriminator.
const P2C_NAME_OFFSET: usize = 8;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "P2C (post-pack confirmation) prepaid subscription escrow CLI for Users/Consumers",
    arg_required_else_help = true,
    color = clap::ColorChoice::Always
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the Solana keypair used as funder / manager authority.
    #[arg(short, long, global = true, default_value = "~/.config/solana/id.json")]
    keypair: String,

    /// Solana RPC endpoint or moniker: m, t, d, l.
    #[arg(
        short,
        long,
        global = true,
        default_value = "t",
        value_parser = normalize_to_url_if_moniker
    )]
    url: String,

    /// Reward Distribution program ID.
    #[arg(
        short,
        long,
        required = true,
        value_parser = parse_pubkey,
        help = "Reward Distribution Program ID [testnet: A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB, mainnet-beta: RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB]"
    )]
    program_id: Pubkey,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch and display one P2C subscription escrow.
    GetAccount(AccountArgs),
    /// List every P2C account for a service name.
    GetAllAccounts(NameArgs),
    /// Fund prepaid balance (does not clear deficit).
    Fund(FundArgs),
    /// Record stake + amount due for an epoch (manager).
    Record(RecordArgs),
    /// Claim epoch fee from prepaid (manager; partial OK; optional force-close with deficit).
    Claim(ClaimArgs),
    /// Clear open deficit (funder transfers; pays commission + identity).
    ClearDeficit(ClearDeficitArgs),
}

#[derive(Args)]
struct NameArgs {
    /// P2C service name (PDA seed).
    #[arg(long = "name", alias = "revenue-name", required = true)]
    name: String,
}

#[derive(Args)]
struct AccountArgs {
    #[command(flatten)]
    name: NameArgs,

    #[arg(short = 'v', long = "vote-pubkey", required = true, value_parser = parse_pubkey)]
    vote_pubkey: Pubkey,
}

#[derive(Args)]
struct FundArgs {
    #[command(flatten)]
    account: AccountArgs,

    #[arg(short = 'x', long, required = true)]
    amount: u64,
}

#[derive(Args)]
struct RecordArgs {
    #[command(flatten)]
    account: AccountArgs,

    #[arg(short, long, required = true)]
    epoch: u64,

    /// Off-chain stake snapshot for this epoch.
    #[arg(long, required = true)]
    stake: u64,

    /// Fee due for the epoch (lamports).
    #[arg(short = 'x', long = "amount-due", required = true)]
    amount_due: u64,
}

#[derive(Args)]
struct ClaimArgs {
    #[command(flatten)]
    account: AccountArgs,

    #[arg(short, long, required = true)]
    epoch: u64,

    /// Validator identity receiving the non-commission share.
    #[arg(long = "validator-identity", required = true, value_parser = parse_pubkey)]
    validator_identity: Pubkey,

    /// Close epoch even when underfunded; remaining due is added to deficit.
    #[arg(long = "force-claim", default_value_t = false)]
    force_claim: bool,
}

#[derive(Args)]
struct ClearDeficitArgs {
    #[command(flatten)]
    account: AccountArgs,

    #[arg(short = 'x', long)]
    amount: Option<u64>,

    #[arg(long = "validator-identity", required = true, value_parser = parse_pubkey)]
    validator_identity: Pubkey,

    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn name_to_bytes(name: &str) -> Result<[u8; 32], String> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return Err("name must not be empty".into());
    }
    if bytes.len() > 32 {
        return Err(format!("name exceeds 32 bytes (got {})", bytes.len()));
    }
    let mut out = [0u8; 32];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(out)
}

fn name_to_string(name: &[u8; 32]) -> String {
    let end = name.iter().position(|&b| b == 0).unwrap_or(32);
    String::from_utf8_lossy(&name[..end]).into_owned()
}

fn print_heading(title: &str) {
    println!("\n{}", title.bold().underline());
}

fn print_field(icon: ColoredString, label: &str, value: impl std::fmt::Display) {
    println!("   {} {:<18} {}", icon, label.dimmed(), value);
}

fn format_total_with_sol(lamports: u64) -> String {
    format!("{} lamports ({:.9} SOL)", lamports, lamports as f64 / 1e9)
}

fn load_p2c(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    name: &str,
    vote: Pubkey,
) -> CliResult<(Pubkey, P2CSubscriptionAccount)> {
    let name_bytes = name_to_bytes(name)?;
    let address = derive_p2c_subscription_address(&program_id, &name_bytes, &vote).0;
    let raw = rpc_client
        .get_account(&address)
        .map_err(|_| format!("P2C account does not exist at {address}"))?;
    if raw.owner != program_id {
        return Err(format!("account {address} owner mismatch").into());
    }
    let mut data = raw.data.as_slice();
    let account = P2CSubscriptionAccount::try_deserialize(&mut data)
        .map_err(|e| format!("failed to deserialize P2C account: {e}"))?;
    Ok((address, account))
}

fn status_name(s: P2CSubscriptionStatus) -> &'static str {
    match s {
        P2CSubscriptionStatus::Active => "Active",
        P2CSubscriptionStatus::InGrace => "InGrace",
        P2CSubscriptionStatus::Suspended => "Suspended",
    }
}

fn display_p2c(address: Pubkey, account: &P2CSubscriptionAccount, balance: u64) {
    print_heading("P2C Subscription Escrow");
    print_field("🔗".cyan(), "Pubkey:", address.to_string().bold().green());
    print_field(
        "📝".cyan(),
        "Name:",
        name_to_string(&account.name).magenta(),
    );
    print_field("🔑".red(), "Vote:", account.validator_vote.to_string());
    print_field(
        "🔏".magenta(),
        "Manager:",
        account.manager_authority.to_string(),
    );
    print_field(
        "🔏".magenta(),
        "Record auth (BR):",
        account.record_authority.to_string(),
    );
    print_field(
        "📊".cyan(),
        "Status:",
        status_name(account.status).magenta(),
    );
    print_field(
        "💰".yellow(),
        "Deficit:",
        format_total_with_sol(account.deficit).yellow(),
    );
    print_field(
        "📝".cyan(),
        "Commission bps:",
        account.commission_bps.to_string().blue(),
    );
    print_field(
        "💰".green(),
        "Balance:",
        format_total_with_sol(balance).yellow(),
    );
    print_field(
        "📦".cyan(),
        "Ledger epochs:",
        account.ledger.entries.len().to_string().blue(),
    );
    for e in &account.ledger.entries {
        println!(
            "      epoch {:>6}: due={} deducted={} claimed={} stake={}",
            e.epoch, e.amount_due, e.amount_deducted, e.claimed, e.stake
        );
    }
}

fn process_get_account(rpc_client: &RpcClient, program_id: Pubkey, args: AccountArgs) -> CliResult {
    let (address, account) = load_p2c(rpc_client, program_id, &args.name.name, args.vote_pubkey)?;
    let balance = rpc_client.get_balance(&address)?;
    display_p2c(address, &account, balance);
    Ok(())
}

fn process_get_all_accounts(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: NameArgs,
) -> CliResult {
    let name = name_to_bytes(&args.name)?;
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        P2C_NAME_OFFSET,
        name.to_vec(),
    ))];
    let accounts = rpc_client.get_program_accounts_with_config(
        &program_id,
        RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            },
            with_context: None,
            sort_results: None,
        },
    )?;

    print_heading("P2C Subscription Accounts");
    print_field("📝".cyan(), "Name:", args.name.magenta());
    print_field("📦".cyan(), "Found:", accounts.len().to_string().magenta());

    for (address, raw) in accounts {
        let mut data = raw.data.as_slice();
        if let Ok(account) = P2CSubscriptionAccount::try_deserialize(&mut data) {
            if account.name == name {
                println!();
                display_p2c(address, &account, raw.lamports);
            }
        }
    }
    Ok(())
}

fn process_fund(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: FundArgs,
) -> CliResult {
    if args.amount == 0 {
        return Err("amount must be greater than zero".into());
    }
    let (address, _) = load_p2c(
        &rpc_client,
        program_id,
        &args.account.name.name,
        args.account.vote_pubkey,
    )?;
    let funder = parse_keypair(keypair_path)?;
    let instruction = fund_p2c_subscription_ix(
        program_id,
        FundP2CSubscriptionArgs {
            amount: args.amount,
        },
        FundP2CSubscriptionAccounts {
            p2c_subscription_account: address,
            funder: funder.pubkey(),
            system_program: system_program::ID,
        },
    );
    print_heading("P2C Fund Prepaid");
    print_field("🔗".cyan(), "Account:", address.to_string().bold().green());
    print_field(
        "💰".green(),
        "Amount:",
        format_total_with_sol(args.amount).yellow(),
    );
    sign_and_send_transaction(rpc_client, instruction, &funder)
}

fn process_record(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: RecordArgs,
) -> CliResult {
    let (address, account) = load_p2c(
        &rpc_client,
        program_id,
        &args.account.name.name,
        args.account.vote_pubkey,
    )?;
    let authority = parse_keypair(keypair_path)?;
    if authority.pubkey() != account.manager_authority {
        return Err(format!(
            "keypair is not manager_authority {}",
            account.manager_authority
        )
        .into());
    }
    let instruction = record_p2c_subscription_ix(
        program_id,
        RecordP2CSubscriptionArgs {
            epoch: args.epoch,
            stake: args.stake,
            amount_due: args.amount_due,
        },
        RecordP2CSubscriptionAccounts {
            p2c_subscription_account: address,
            manager_authority: authority.pubkey(),
        },
    );
    print_heading("P2C Record Subscription Charge");
    print_field("🔗".cyan(), "Account:", address.to_string().bold().green());
    print_field("🕒".cyan(), "Epoch:", args.epoch.to_string().blue());
    print_field("📝".cyan(), "Stake:", args.stake.to_string().blue());
    print_field(
        "💰".green(),
        "Due:",
        format_total_with_sol(args.amount_due).yellow(),
    );
    sign_and_send_transaction(rpc_client, instruction, &authority)
}

fn process_claim(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: ClaimArgs,
) -> CliResult {
    let (address, account) = load_p2c(
        &rpc_client,
        program_id,
        &args.account.name.name,
        args.account.vote_pubkey,
    )?;
    let manager = parse_keypair(keypair_path)?;
    if manager.pubkey() != account.manager_authority {
        return Err(format!(
            "keypair is not manager_authority {}",
            account.manager_authority
        )
        .into());
    }
    let instruction = claim_epoch_p2c_subscription_ix(
        program_id,
        ClaimEpochP2CSubscriptionArgs {
            epoch: args.epoch,
            force_claim: args.force_claim,
        },
        ClaimEpochP2CSubscriptionAccounts {
            p2c_subscription_account: address,
            commission_account: account.commission_account,
            validator_identity: args.validator_identity,
            manager_authority: manager.pubkey(),
        },
    );
    print_heading("P2C Claim Epoch");
    print_field("🔗".cyan(), "Account:", address.to_string().bold().green());
    print_field("🕒".cyan(), "Epoch:", args.epoch.to_string().blue());
    print_field(
        "⚡".yellow(),
        "Force claim:",
        args.force_claim.to_string().blue(),
    );
    sign_and_send_transaction(rpc_client, instruction, &manager)
}

fn process_clear_deficit(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: ClearDeficitArgs,
) -> CliResult {
    let (address, account) = load_p2c(
        &rpc_client,
        program_id,
        &args.account.name.name,
        args.account.vote_pubkey,
    )?;
    if account.deficit == 0 {
        return Err("P2C account has no open deficit".into());
    }
    let applied = args.amount.unwrap_or(account.deficit).min(account.deficit);
    if applied == 0 {
        return Err("amount must be greater than zero".into());
    }
    let funder = parse_keypair(keypair_path)?;
    let instruction = clear_p2c_deficit_ix(
        program_id,
        ClearP2CDeficitArgs { amount: applied },
        ClearP2CDeficitAccounts {
            p2c_subscription_account: address,
            commission_account: account.commission_account,
            validator_identity: args.validator_identity,
            funder: funder.pubkey(),
            system_program: system_program::ID,
        },
    );
    print_heading("P2C Clear Deficit");
    print_field("🔗".cyan(), "Account:", address.to_string().bold().green());
    print_field(
        "💰".yellow(),
        "Open deficit:",
        format_total_with_sol(account.deficit).yellow(),
    );
    print_field(
        "💰".green(),
        "Clear amount:",
        format_total_with_sol(applied).yellow(),
    );
    if args.dry_run {
        println!(
            "\n   {}",
            "Dry run only — no transaction was sent.".yellow()
        );
        return Ok(());
    }
    sign_and_send_transaction(rpc_client, instruction, &funder)
}

fn main() -> CliResult {
    let cli = Cli::parse();
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.url,
        CommitmentConfig::confirmed(),
    ));

    match cli.command {
        Commands::GetAccount(args) => process_get_account(&rpc_client, cli.program_id, args),
        Commands::GetAllAccounts(args) => {
            process_get_all_accounts(&rpc_client, cli.program_id, args)
        }
        Commands::Fund(args) => process_fund(rpc_client, cli.program_id, &cli.keypair, args),
        Commands::Record(args) => process_record(rpc_client, cli.program_id, &cli.keypair, args),
        Commands::Claim(args) => process_claim(rpc_client, cli.program_id, &cli.keypair, args),
        Commands::ClearDeficit(args) => {
            process_clear_deficit(rpc_client, cli.program_id, &cli.keypair, args)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_zero_padded() {
        let name = name_to_bytes("p2c-svc").unwrap();
        assert_eq!(&name[..7], b"p2c-svc");
        assert!(name[7..].iter().all(|byte| *byte == 0));
    }
}
