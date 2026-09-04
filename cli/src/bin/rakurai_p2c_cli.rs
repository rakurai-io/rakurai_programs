use {
    anchor_lang::AccountDeserialize,
    clap::{Args, Parser, Subcommand},
    colored::{ColoredString, Colorize},
    rakurai_cli::{
        from_anchor_pubkey, normalize_to_url_if_moniker, parse_keypair, parse_pubkey,
        sign_and_send_instructions, sign_and_send_transaction, to_anchor_pubkey,
        to_solana_instruction,
    },
    reward_distribution::{
        sdk::{
            derive_p2c_config_address, derive_p2c_subscription_address,
            instruction::{
                claim_epoch_p2c_subscription_ix, clear_p2c_deficit_ix, fund_p2c_subscription_ix,
                initialize_p2c_subscription_account_ix, record_p2c_subscription_ix,
                ClaimEpochP2CSubscriptionAccounts, ClaimEpochP2CSubscriptionArgs,
                ClearP2CDeficitAccounts, ClearP2CDeficitArgs, FundP2CSubscriptionAccounts,
                FundP2CSubscriptionArgs, InitializeP2CSubscriptionAccountAccounts,
                InitializeP2CSubscriptionAccountArgs, RecordP2CSubscriptionAccounts,
                RecordP2CSubscriptionArgs,
            },
        },
        state::{
            P2CEpochEntry, P2CSubscriptionAccount, P2CSubscriptionStatus, RAKURAI_REVENUE_NAME,
        },
    },
    solana_account_decoder_client_types::UiAccountEncoding,
    solana_commitment_config::CommitmentConfig,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_rpc_client::rpc_client::RpcClient,
    solana_rpc_client_api::{
        config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        filter::{Memcmp, RpcFilterType},
    },
    solana_signer::Signer,
    solana_system_interface::program as system_program,
    std::{error::Error, sync::Arc},
};

type CliResult<T = ()> = Result<T, Box<dyn Error>>;

/// P2C account name starts immediately after the 8-byte Anchor discriminator.
const P2C_NAME_OFFSET: usize = 8;
const DEFAULT_FUND_ALL_BATCH_SIZE: usize = 10;

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
    /// Create a P2C subscription escrow (PSA). Reserved name `rakurai` is blocked.
    #[command(hide = true)]
    CreateAccount(CreateAccountArgs),
    /// Fetch and display one P2C subscription escrow.
    GetAccount(AccountArgs),
    /// List every P2C account for a service name.
    GetAllAccounts(GetAllAccountsArgs),
    /// Fund prepaid balance (does not clear deficit).
    Fund(FundArgs),
    /// Fund shortfalls for every PSA under this service name (past epochs only).
    FundAll(FundAllArgs),
    /// Record stake + amount due for an epoch (manager).
    #[command(hide = true)]
    Record(RecordArgs),
    /// Claim epoch fee from prepaid (manager; partial OK; optional force-close with deficit).
    #[command(hide = true)]
    Claim(ClaimArgs),
    /// Clear open deficit (funder transfers; pays commission + identity).
    #[command(hide = true)]
    ClearDeficit(ClearDeficitArgs),
}

#[derive(Args)]
struct NameArgs {
    /// P2C service name (PDA seed).
    #[arg(long = "name", alias = "revenue-name", required = true)]
    name: String,
}

#[derive(Args)]
struct CreateAccountArgs {
    #[command(flatten)]
    name: NameArgs,

    #[arg(short = 'v', long = "vote-pubkey", required = true, value_parser = parse_pubkey)]
    vote_pubkey: Pubkey,

    /// Preview without sending a transaction.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Args)]
struct AccountArgs {
    #[command(flatten)]
    name: NameArgs,

    #[arg(short = 'v', long = "vote-pubkey", required = true, value_parser = parse_pubkey)]
    vote_pubkey: Pubkey,

    /// Show manager/auth info and per-epoch ledger breakdown.
    #[arg(long, default_value_t = false)]
    detail: bool,
}

#[derive(Args)]
struct GetAllAccountsArgs {
    #[command(flatten)]
    name: NameArgs,

    /// Show per-epoch ledger for each account.
    #[arg(long, default_value_t = false)]
    detail: bool,
}

#[derive(Args)]
struct FundArgs {
    #[command(flatten)]
    account: AccountArgs,

    #[arg(short = 'x', long, required = true)]
    amount: u64,
}

#[derive(Args)]
struct FundAllArgs {
    #[command(flatten)]
    name: NameArgs,

    /// Fund instructions per transaction. Default: 10.
    #[arg(long, default_value_t = DEFAULT_FUND_ALL_BATCH_SIZE)]
    batch_size: usize,

    /// Preview shortfalls without sending transactions.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
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

fn reject_reserved_rakurai_name(name: &[u8; 32]) -> CliResult {
    if *name == RAKURAI_REVENUE_NAME {
        return Err(
            "reserved name `rakurai` cannot be used to create partner PSA/MCA accounts".into(),
        );
    }
    Ok(())
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
    let address = from_anchor_pubkey(
        derive_p2c_subscription_address(
            &to_anchor_pubkey(program_id),
            &name_bytes,
            &to_anchor_pubkey(vote),
        )
        .0,
    );
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

fn load_all_p2c_by_name(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    name: &str,
) -> CliResult<Vec<(Pubkey, P2CSubscriptionAccount, u64)>> {
    let name_bytes = name_to_bytes(name)?;
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        P2C_NAME_OFFSET,
        name_bytes.to_vec(),
    ))];
    let accounts = rpc_client
        .get_program_ui_accounts_with_config(
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
        )?
        .into_iter()
        .filter_map(|(address, ui)| ui.to_account().map(|raw| (address, raw)))
        .collect::<Vec<_>>();

    let mut out = Vec::new();
    for (address, raw) in accounts {
        let mut data = raw.data.as_slice();
        if let Ok(account) = P2CSubscriptionAccount::try_deserialize(&mut data) {
            if account.name == name_bytes {
                out.push((address, account, raw.lamports));
            }
        }
    }
    out.sort_by_key(|(_, account, _)| account.validator_vote.to_string());
    Ok(out)
}

fn short_pubkey(s: &str) -> String {
    if s.len() <= 12 {
        return s.to_string();
    }
    format!("{}....{}", &s[..6], &s[s.len() - 6..])
}

fn status_name(s: P2CSubscriptionStatus) -> &'static str {
    match s {
        P2CSubscriptionStatus::Active => "Active",
        P2CSubscriptionStatus::InGrace => "InGrace",
        P2CSubscriptionStatus::Suspended => "Suspended",
    }
}

fn balance_after_rent(account: &P2CSubscriptionAccount, lamports: u64) -> u64 {
    let space = P2CSubscriptionAccount::space_for(account.max_epoch_entries as usize);
    lamports.saturating_sub(Rent::default().minimum_balance(space))
}

/// Remaining fee for an unclaimed epoch (`due - deducted`). Claimed rows are settled (shortfall → deficit).
fn epoch_owed(entry: &P2CEpochEntry) -> u64 {
    if entry.claimed {
        0
    } else {
        entry.amount_due.saturating_sub(entry.amount_deducted)
    }
}

/// Pending owed excluding the in-progress cluster epoch (not actionable yet).
fn actionable_owed(account: &P2CSubscriptionAccount, current_epoch: u64) -> Vec<(u64, u64)> {
    let mut pending: Vec<_> = account
        .ledger
        .entries
        .iter()
        .filter_map(|e| {
            if e.epoch == current_epoch {
                return None;
            }
            let owed = epoch_owed(e);
            (owed > 0).then_some((e.epoch, owed))
        })
        .collect();
    pending.sort_unstable_by_key(|(epoch, _)| *epoch);
    pending
}

fn print_underfunded_alert(available: u64, pending: &[(u64, u64)]) {
    let owed: u64 = pending.iter().map(|(_, o)| *o).sum();
    if owed == 0 {
        println!("   {} {}", "✅".green(), "Nothing pending".green());
        return;
    }
    // Funded enough for past epochs — no warn (current epoch is excluded above).
    if available >= owed {
        return;
    }
    println!(
        "   {} {}",
        "❌".red(),
        format!(
            "{} epoch(s) pending  — fund them at the earliest to continue using services.",
            pending.len(),
        )
    );
}

fn display_p2c(
    address: Pubkey,
    account: &P2CSubscriptionAccount,
    balance: u64,
    current_epoch: u64,
    detail: bool,
) {
    let available = balance_after_rent(account, balance);
    print_heading("P2C Subscription Escrow");
    print_field("🔗".cyan(), "Pubkey:", address.to_string().bold().green());
    print_field(
        "📝".cyan(),
        "Name:",
        name_to_string(&account.name).magenta(),
    );
    print_field("🔑".red(), "Vote:", account.validator_vote.to_string());

    println!();
    print_field(
        "📊".cyan(),
        "Status:",
        status_name(account.status).magenta(),
    );
    print_field(
        "💰".green(),
        "Balance:",
        format_total_with_sol(available).yellow(),
    );

    let pending = actionable_owed(account, current_epoch);
    let total_owed: u64 = pending.iter().map(|(_, o)| *o).sum();

    print_field(
        "💸".yellow(),
        "Total owed:",
        format_total_with_sol(total_owed).yellow(),
    );

    if account.deficit > 0 {
        print_field(
            "⚠️".red(),
            "Deficit:",
            format_total_with_sol(account.deficit).yellow(),
        );
        println!(
            "   {} {}",
            "❌".red(),
            "Clear this deficit at the earliest to continue using services.".red()
        );
    }

    print_underfunded_alert(available, &pending);

    if !detail {
        println!(
            "\n   {}",
            "Use --detail to see manager, auth, and per-epoch breakdown.".dimmed()
        );
        return;
    }

    println!();
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
        "📝".cyan(),
        "Commission:",
        format!("{:.2}%", account.commission_bps as f64 / 100.0).blue(),
    );
    print_field(
        "📝".cyan(),
        "Grace epochs:",
        account.grace_epochs.to_string().blue(),
    );
    print_field(
        "📝".cyan(),
        "Unpaid streak:",
        account.unpaid_streak.to_string().blue(),
    );

    println!();
    println!("   {} {}", "📋".cyan(), "Epoch Details:".bold());
    let mut entries: Vec<_> = account.ledger.entries.iter().collect();
    entries.sort_unstable_by_key(|e| e.epoch);
    if entries.is_empty() {
        println!("   {}", "(no epoch entries)".dimmed());
    } else {
        for e in entries {
            println!(
                "      epoch {:>6}: due={:<12} deducted={:<12} owed={:<12} claimed={}",
                e.epoch.to_string().blue(),
                e.amount_due.to_string().yellow(),
                e.amount_deducted.to_string().yellow(),
                epoch_owed(e).to_string().yellow(),
                e.claimed.to_string().blue(),
            );
        }
    }
}

fn process_create_account(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: CreateAccountArgs,
) -> CliResult {
    let name = name_to_bytes(&args.name.name)?;
    reject_reserved_rakurai_name(&name)?;

    let payer = parse_keypair(keypair_path)?;
    let program_id_a = to_anchor_pubkey(program_id);
    let vote_a = to_anchor_pubkey(args.vote_pubkey);
    let (p2c_config_a, _) = derive_p2c_config_address(&program_id_a);
    let (address_a, bump) = derive_p2c_subscription_address(&program_id_a, &name, &vote_a);
    let p2c_config = from_anchor_pubkey(p2c_config_a);
    let address = from_anchor_pubkey(address_a);

    if rpc_client.get_account(&address).is_ok() {
        return Err(format!("PSA already exists at {address}").into());
    }
    if rpc_client.get_account(&p2c_config).is_err() {
        return Err(format!(
            "P2C config does not exist at {p2c_config}; Rakurai must initialize P2CConfig first"
        )
        .into());
    }

    print_heading("Create P2C Subscription Escrow (PSA)");
    print_field("🔗".cyan(), "Pubkey:", address.to_string().bold().green());
    print_field("📝".cyan(), "Name:", args.name.name.magenta());
    print_field("🔑".red(), "Vote:", args.vote_pubkey.to_string());
    print_field("🔑".red(), "Payer:", payer.pubkey().to_string());
    print_field(
        "📦".cyan(),
        "Defaults from:",
        format!("P2CConfig {p2c_config}"),
    );

    if args.dry_run {
        println!(
            "\n   {}",
            "Dry run only — no transaction was sent.".yellow()
        );
        return Ok(());
    }

    let instruction = to_solana_instruction(initialize_p2c_subscription_account_ix(
        program_id_a,
        InitializeP2CSubscriptionAccountArgs { name, bump },
        InitializeP2CSubscriptionAccountAccounts {
            p2c_subscription_account: address_a,
            p2c_config: p2c_config_a,
            validator_vote_account: vote_a,
            payer: to_anchor_pubkey(payer.pubkey()),
            system_program: to_anchor_pubkey(system_program::id()),
        },
    ));
    sign_and_send_transaction(rpc_client, instruction, &payer)
}

fn process_get_account(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: &AccountArgs,
) -> CliResult {
    let (address, account) = load_p2c(rpc_client, program_id, &args.name.name, args.vote_pubkey)?;
    let balance = rpc_client.get_balance(&address)?;
    let current_epoch = rpc_client.get_epoch_info()?.epoch;
    display_p2c(address, &account, balance, current_epoch, args.detail);
    Ok(())
}

fn process_get_all_accounts(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: GetAllAccountsArgs,
) -> CliResult {
    let accounts = load_all_p2c_by_name(rpc_client, program_id, &args.name.name)?;
    let current_epoch = rpc_client.get_epoch_info()?.epoch;

    print_heading("P2C Subscription Accounts");
    print_field("📝".cyan(), "Name:", args.name.name.magenta());
    print_field("📦".cyan(), "Found:", accounts.len().to_string().magenta());

    for (address, account, lamports) in accounts {
        display_p2c(address, &account, lamports, current_epoch, args.detail);
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
    let instruction = to_solana_instruction(fund_p2c_subscription_ix(
        to_anchor_pubkey(program_id),
        FundP2CSubscriptionArgs {
            amount: args.amount,
        },
        FundP2CSubscriptionAccounts {
            p2c_subscription_account: to_anchor_pubkey(address),
            funder: to_anchor_pubkey(funder.pubkey()),
            system_program: to_anchor_pubkey(system_program::id()),
        },
    ));
    print_heading("P2C Fund Prepaid");
    print_field("🔗".cyan(), "Account:", address.to_string().bold().green());
    print_field(
        "💰".green(),
        "Amount:",
        format_total_with_sol(args.amount).yellow(),
    );
    sign_and_send_transaction(rpc_client, instruction, &funder)
}

struct PendingFund {
    vote: Pubkey,
    address: Pubkey,
    owed: u64,
    available: u64,
    shortfall: u64,
    instruction: Instruction,
}

fn process_fund_all(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: FundAllArgs,
) -> CliResult {
    if args.batch_size == 0 {
        return Err("--batch-size must be at least 1".into());
    }

    let accounts = load_all_p2c_by_name(&rpc_client, program_id, &args.name.name)?;
    let funder = parse_keypair(keypair_path)?;
    let current_epoch = rpc_client.get_epoch_info()?.epoch;

    let mut jobs = Vec::new();
    for (address, account, lamports) in &accounts {
        let available = balance_after_rent(account, *lamports);
        let pending = actionable_owed(account, current_epoch);
        let owed: u64 = pending.iter().map(|(_, o)| *o).sum();
        if owed == 0 || available >= owed {
            continue;
        }
        let shortfall = owed.saturating_sub(available);
        let instruction = to_solana_instruction(fund_p2c_subscription_ix(
            to_anchor_pubkey(program_id),
            FundP2CSubscriptionArgs { amount: shortfall },
            FundP2CSubscriptionAccounts {
                p2c_subscription_account: to_anchor_pubkey(*address),
                funder: to_anchor_pubkey(funder.pubkey()),
                system_program: to_anchor_pubkey(system_program::id()),
            },
        ));
        jobs.push(PendingFund {
            vote: from_anchor_pubkey(account.validator_vote),
            address: *address,
            owed,
            available,
            shortfall,
            instruction,
        });
    }

    let total_shortfall: u64 = jobs.iter().map(|j| j.shortfall).sum();

    print_heading("Fund All P2C Shortfalls");
    print_field("📝".cyan(), "Name:", args.name.name.magenta());
    print_field(
        "📦".cyan(),
        "Accounts:",
        accounts.len().to_string().magenta(),
    );
    print_field(
        "🕒".cyan(),
        "Underfunded:",
        jobs.len().to_string().magenta(),
    );
    print_field(
        "💸".yellow(),
        "Total fund:",
        format_total_with_sol(total_shortfall).yellow(),
    );
    print_field(
        "📦".cyan(),
        "Ix/txn:",
        args.batch_size.to_string().magenta(),
    );
    print_field("🔑".red(), "Funder:", funder.pubkey().to_string());
    print_field(
        "🕒".cyan(),
        "Excludes epoch:",
        current_epoch.to_string().blue(),
    );

    if jobs.is_empty() {
        println!(
            "\n   {}",
            "Nothing underfunded to fund (past epochs only).".green()
        );
        return Ok(());
    }

    println!();
    for job in &jobs {
        println!(
            "   {}  owed {}  bal {}  fund {}",
            short_pubkey(&job.vote.to_string()).cyan(),
            format_total_with_sol(job.owed).yellow(),
            format_total_with_sol(job.available).yellow(),
            format_total_with_sol(job.shortfall).yellow(),
        );
    }

    if args.dry_run {
        println!(
            "\n   {}",
            "Dry run only — no transactions were sent.".yellow()
        );
        return Ok(());
    }

    println!();
    let mut sent = 0usize;
    for chunk in jobs.chunks(args.batch_size) {
        let instructions: Vec<_> = chunk.iter().map(|job| job.instruction.clone()).collect();
        print_heading(&format!("Sending batch of {} fund(s)", instructions.len()));
        for job in chunk {
            println!(
                "   {}  {}  +{}",
                short_pubkey(&job.vote.to_string()).cyan(),
                short_pubkey(&job.address.to_string()).dimmed(),
                format_total_with_sol(job.shortfall).yellow(),
            );
        }
        sign_and_send_instructions(rpc_client.clone(), &instructions, &funder)?;
        sent += instructions.len();
    }

    println!(
        "\n   {} {}",
        "✅".green(),
        format!("Funded {sent} underfunded P2C account(s).").green()
    );
    Ok(())
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
    let manager = from_anchor_pubkey(account.manager_authority);
    if authority.pubkey() != manager {
        return Err(format!("keypair is not manager_authority {manager}").into());
    }
    let instruction = to_solana_instruction(record_p2c_subscription_ix(
        to_anchor_pubkey(program_id),
        RecordP2CSubscriptionArgs {
            epoch: args.epoch,
            stake: args.stake,
            amount_due: args.amount_due,
        },
        RecordP2CSubscriptionAccounts {
            p2c_subscription_account: to_anchor_pubkey(address),
            manager_authority: to_anchor_pubkey(authority.pubkey()),
        },
    ));
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
    let manager_authority = from_anchor_pubkey(account.manager_authority);
    if manager.pubkey() != manager_authority {
        return Err(format!("keypair is not manager_authority {manager_authority}").into());
    }
    let instruction = to_solana_instruction(claim_epoch_p2c_subscription_ix(
        to_anchor_pubkey(program_id),
        ClaimEpochP2CSubscriptionArgs {
            epoch: args.epoch,
            force_claim: args.force_claim,
        },
        ClaimEpochP2CSubscriptionAccounts {
            p2c_subscription_account: to_anchor_pubkey(address),
            commission_account: account.commission_account,
            validator_identity: to_anchor_pubkey(args.validator_identity),
            manager_authority: to_anchor_pubkey(manager.pubkey()),
        },
    ));
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
    let instruction = to_solana_instruction(clear_p2c_deficit_ix(
        to_anchor_pubkey(program_id),
        ClearP2CDeficitArgs { amount: applied },
        ClearP2CDeficitAccounts {
            p2c_subscription_account: to_anchor_pubkey(address),
            commission_account: account.commission_account,
            validator_identity: to_anchor_pubkey(args.validator_identity),
            funder: to_anchor_pubkey(funder.pubkey()),
            system_program: to_anchor_pubkey(system_program::id()),
        },
    ));
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
        Commands::CreateAccount(args) => {
            process_create_account(rpc_client, cli.program_id, &cli.keypair, args)
        }
        Commands::GetAccount(ref args) => process_get_account(&rpc_client, cli.program_id, args),
        Commands::GetAllAccounts(args) => {
            process_get_all_accounts(&rpc_client, cli.program_id, args)
        }
        Commands::Fund(args) => process_fund(rpc_client, cli.program_id, &cli.keypair, args),
        Commands::FundAll(args) => process_fund_all(rpc_client, cli.program_id, &cli.keypair, args),
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

    #[test]
    fn rejects_reserved_rakurai_name() {
        let name = name_to_bytes("rakurai").unwrap();
        assert!(reject_reserved_rakurai_name(&name).is_err());
    }
}
